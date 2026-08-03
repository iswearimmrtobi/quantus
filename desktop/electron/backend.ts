import { ChildProcess, spawn } from 'node:child_process';
import { existsSync, readdirSync } from 'node:fs';
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

    const jar = findJar();
    if (!jar) {
      throw new Error(
        'Das Backend-Jar wurde nicht gefunden. Im Projekt einmal `./gradlew bootJar` ausführen ' +
          '— oder das Backend separat mit `./gradlew bootRun` starten.',
      );
    }

    const java = findJava();
    console.log(`[quantus] starting backend: ${java} -jar ${jar}`);
    // stdin bleibt als Pipe offen und wird nie beschrieben: stirbt diese App, schließt das
    // Betriebssystem die Pipe, und der Watchdog im Backend beendet sich selbst. Das deckt
    // die Fälle ab, in denen `stop()` nicht mehr zum Zug kommt (SIGKILL, Absturz).
    this.child = spawn(java, ['-jar', jar, `--server.port=${this.port}`, '--quantus.parent-watch=true'], {
      stdio: ['pipe', 'pipe', 'pipe'],
    });
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
    const stopped = await waitFor(() => child.exitCode !== null || child.killed, 5000, 100);
    if (!stopped) {
      console.warn('[quantus] backend did not stop in time — killing it');
      child.kill('SIGKILL');
    }
    this.child = null;
  }

  private async waitUntilUp(): Promise<void> {
    const up = await waitFor(() => this.isUp(500), STARTUP_TIMEOUT_MS, 250);
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

/**
 * The backend jar: next to the app when packaged, from the Gradle build directory during
 * development. `-plain.jar` is Gradle's jar without dependencies and cannot be started.
 */
function findJar(): string | null {
  if (app.isPackaged) {
    const bundled = join(process.resourcesPath, 'backend', 'quantus.jar');
    return existsSync(bundled) ? bundled : null;
  }

  const libs = resolve(__dirname, '..', '..', 'build', 'libs');
  if (!existsSync(libs)) return null;
  const jars = readdirSync(libs).filter((name) => name.endsWith('.jar') && !name.endsWith('-plain.jar'));
  return jars.length > 0 ? join(libs, jars.sort().at(-1)!) : null;
}

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
