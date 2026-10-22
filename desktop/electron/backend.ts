import { ChildProcess, spawn } from 'node:child_process';
import { existsSync, mkdirSync, readdirSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { app } from 'electron';

/**
 * The Kotlin backend as a child process (N2a).
 *
 * If something already answers on the port — `./gradlew bootRun` during development, or a
 * self-hosted instance (N4) — the app attaches to it and leaves it alone; only a backend it
 * started itself does it stop again.
 */
export class Backend {
  private child: ChildProcess | null = null;
  private stopping = false;

  constructor(readonly url: string, private readonly port: number) {}

  /** True when the backend answers `/api/health`. */
  async isUp(timeoutMs = 1000): Promise<boolean> {
    try {
      const response = await fetch(`${this.url}/api/health`, { signal: AbortSignal.timeout(timeoutMs) });
      return response.ok;
    } catch {
      return false;
    }
  }

  /**
   * Makes sure a backend is running: attaches to a live one, otherwise starts the jar and
   * waits for its health check. Rejects with a message meant for the user.
   */
  async start(): Promise<void> {
    if (await this.isUp()) {
      console.log(`[quantus] backend already running on ${this.url} — attaching`);
      return;
    }

    const backend = findBackend();
    if (!backend) {
      throw new Error(
        'Das Backend-Jar wurde nicht gefunden. Im Projekt einmal `./gradlew desktopResources` ' +
          'ausführen — oder das Backend separat mit `./gradlew bootRun` starten.',
      );
    }

    const java = findJava();
    console.log(`[quantus] starting backend: ${java} -jar ${backend.jar} (in ${backend.directory})`);
    // stdin bleibt als Pipe offen und wird nie beschrieben: stirbt diese App, schließt das
    // Betriebssystem die Pipe, und der Watchdog im Backend beendet sich selbst. Das deckt
    // die Fälle ab, in denen `stop()` nicht mehr zum Zug kommt (SIGKILL, Absturz).
    this.child = spawn(
      java,
      [
        ...backend.vmOptions,
        '-jar',
        backend.jar,
        `--server.port=${this.port}`,
        '--quantus.parent-watch=true',
      ],
      // Klassenpfad und CDS-Archiv stehen relativ zu diesem Verzeichnis — siehe `findBackend`.
      { cwd: backend.directory, stdio: ['pipe', 'pipe', 'pipe'] },
    );
    this.child.stdout?.on('data', (chunk) => process.stdout.write(`[backend] ${chunk}`));
    this.child.stderr?.on('data', (chunk) => process.stderr.write(`[backend] ${chunk}`));
    this.child.on('exit', (code) => {
      const child = this.child;
      this.child = null;
      if (!this.stopping && child) {
        console.error(`[quantus] backend exited unexpectedly (code ${code})`);
      }
    });

    await this.waitUntilUp();
  }

  /** Ends a backend this app started; an attached one keeps running. */
  async stop(): Promise<void> {
    const child = this.child;
    if (!child || child.exitCode !== null) return;

    this.stopping = true;
    child.kill('SIGTERM');
    // Generous, because the JVM writes its shared archive on the way out (see
    // `sharedArchiveOptions`) — the first shutdown after an update takes a few seconds.
    const stopped = await waitFor(() => child.exitCode !== null || child.killed, SHUTDOWN_TIMEOUT_MS, 50);
    if (!stopped) {
      console.warn('[quantus] backend did not stop in time — killing it');
      child.kill('SIGKILL');
    }
    this.child = null;
  }

  private async waitUntilUp(): Promise<void> {
    const up = await waitFor(() => this.isUp(500), STARTUP_TIMEOUT_MS, 50);
    if (!up) {
      await this.stop();
      throw new Error(
        `Das Backend hat sich innerhalb von ${STARTUP_TIMEOUT_MS / 1000} Sekunden nicht gemeldet ` +
          `(${this.url}). Läuft eine passende Java-Version?`,
      );
    }
  }
}

const STARTUP_TIMEOUT_MS = 60_000;
const SHUTDOWN_TIMEOUT_MS = 20_000;

/** Polls [check] until it holds or the timeout is up. */
export async function waitFor(
  check: () => boolean | Promise<boolean>,
  timeoutMs: number,
  intervalMs: number,
): Promise<boolean> {
  const deadline = Date.now() + timeoutMs;
  for (;;) {
    if (await check()) return true;
    if (Date.now() >= deadline) return false;
    await new Promise((done) => setTimeout(done, intervalMs));
  }
}

/** Where the backend lives and how its JVM has to be started. */
interface BackendLocation {
  /** The working directory the JVM runs in — the jar path is relative to it. */
  directory: string;
  jar: string;
  vmOptions: string[];
}

/**
 * The backend: the unpacked distribution when there is one, otherwise the fat jar from the
 * Gradle build directory.
 *
 * `./gradlew desktopResources` lays the backend out as a plain jar with a `lib/` directory
 * next to it. Spring Boot's fat jar would work too, but every class of it comes out of a zip
 * inside a zip, and that costs a second of every start. `-plain.jar` is Gradle's jar without
 * dependencies and cannot be started at all.
 */
function findBackend(): BackendLocation | null {
  const distribution = app.isPackaged
    ? join(process.resourcesPath, 'backend')
    : resolve(__dirname, '..', '..', 'build', 'dist', 'backend');

  if (existsSync(join(distribution, JAR))) {
    return { directory: distribution, jar: JAR, vmOptions: sharedArchiveOptions() };
  }
  if (app.isPackaged) return null;

  const libs = resolve(__dirname, '..', '..', 'build', 'libs');
  if (!existsSync(libs)) return null;
  const jars = readdirSync(libs).filter((name) => name.endsWith('.jar') && !name.endsWith('-plain.jar'));
  if (jars.length === 0) return null;
  return { directory: libs, jar: jars.sort().at(-1)!, vmOptions: [] };
}

/**
 * Lets the JVM keep a class data sharing archive in the user's cache — the single largest
 * item in the backend's startup time.
 *
 * The archive holds the classes of a previous run already parsed and linked; starting from it
 * takes the backend from three and a half seconds to one and a half. `AutoCreateSharedArchive`
 * makes the JVM look after it on its own: it writes the archive when the app first shuts down
 * and rewrites it whenever it no longer fits the jars, which is exactly what happens after an
 * update. The first start after installing is therefore the slow one, every later start is not.
 *
 * The archive cannot be shipped ready-made. It records size and modification time of every jar
 * it was built from, and the AppImage stamps its whole content with the time it was packed —
 * a time no build step can know beforehand.
 */
function sharedArchiveOptions(): string[] {
  const directory = join(app.getPath('userData'), 'jvm');
  try {
    mkdirSync(directory, { recursive: true });
  } catch (error) {
    console.warn(`[quantus] no cache directory, backend starts the slow way: ${String(error)}`);
    return [];
  }
  return [
    '-XX:+AutoCreateSharedArchive',
    `-XX:SharedArchiveFile=${join(directory, ARCHIVE)}`,
  ];
}

/** The jar name is set by the Gradle build (`BACKEND_JAR`); the archive is ours alone. */
const JAR = 'quantus.jar';
const ARCHIVE = 'quantus.jsa';

/** The JRE: the bundled one (phase 6, jlink), otherwise JAVA_HOME, otherwise the PATH. */
function findJava(): string {
  const executable = process.platform === 'win32' ? 'java.exe' : 'java';

  const bundled = join(process.resourcesPath, 'jre', 'bin', executable);
  if (app.isPackaged && existsSync(bundled)) return bundled;

  const javaHome = process.env.QUANTUS_JAVA_HOME ?? process.env.JAVA_HOME;
  if (javaHome) {
    const fromHome = join(javaHome, 'bin', executable);
    if (existsSync(fromHome)) return fromHome;
  }
  return executable;
}
