import java.net.HttpURLConnection
import java.net.URI
import java.util.concurrent.TimeUnit

/** Der Name, unter dem `desktop/electron/backend.ts` das Backend-Jar sucht. */
val BACKEND_JAR = "quantus.jar"

plugins {
    alias(libs.plugins.kotlin.jvm)
    alias(libs.plugins.kotlin.spring)
    alias(libs.plugins.spring.boot)
    alias(libs.plugins.spring.dependency.management)
}

group = "app.tobiasschmidt"
version = "0.1.0-SNAPSHOT"

val javaToolchains = extensions.getByType<JavaToolchainService>()

java {
    toolchain {
        languageVersion = JavaLanguageVersion.of(21)
    }
}

kotlin {
    compilerOptions {
        freeCompilerArgs.add("-Xjsr305=strict")
    }
}

repositories {
    mavenCentral()
}

dependencies {
    implementation("org.springframework.boot:spring-boot-starter-web")
    implementation("com.fasterxml.jackson.module:jackson-module-kotlin")
    implementation("org.jetbrains.kotlin:kotlin-reflect")

    // CAS: symbolische und numerische Auswertung, beliebige Präzision
    implementation(libs.symja.core)
    // Einheiten: JSR-385-Referenzimplementierung, Dimensionsprüfung
    implementation(libs.indriya)

    testImplementation("org.springframework.boot:spring-boot-starter-test")
    testImplementation(kotlin("test"))
}

tasks.withType<Test> {
    useJUnitPlatform()
    testLogging {
        events("passed", "failed", "skipped")
        showStandardStreams = true
        exceptionFormat = org.gradle.api.tasks.testing.logging.TestExceptionFormat.FULL
    }
}

/**
 * Die Java-Module, die das Backend zur Laufzeit braucht (N2a: jlink-Minimal-Runtime).
 *
 * Die Liste ist von Hand gepflegt statt per `jdeps` ermittelt: das Spring-Boot-Fat-Jar
 * versteckt seine Abhängigkeiten in verschachtelten Jars, die jdeps nicht mitliest. Ob sie
 * stimmt, prüft `verifyRuntime` — das startet das Backend mit genau dieser Runtime.
 */
val backendModules = listOf(
    "java.base",
    // AWT-Klassen: Symja lädt sie beim Plotten, Tomcat für Bildformate
    "java.desktop",
    "java.instrument",
    "java.logging",
    "java.management",
    "java.naming",
    "java.net.http",
    "java.prefs",
    "java.rmi",
    // Symja-Skriptschnittstelle
    "java.scripting",
    "java.security.jgss",
    "java.security.sasl",
    "java.sql",
    "java.transaction.xa",
    "java.xml",
    "jdk.crypto.ec",
    "jdk.management",
    // Sun-Interna, die Netty und Spring über Reflection ansprechen
    "jdk.unsupported",
    // Spring Boot liest das Fat-Jar als ZIP-Dateisystem
    "jdk.zipfs",
)

/** Was die Desktop-App mitbringt: electron-builder kopiert `build/dist` nach `resources/`. */
val distribution = layout.buildDirectory.dir("dist")
val runtimeDirectory = distribution.map { it.dir("jre") }
val backendDirectory = distribution.map { it.dir("backend") }

/** Die Minimal-JRE für den mitgelieferten Backend-Prozess (Phase 6, Packaging). */
val jlinkRuntime by tasks.registering(Exec::class) {
    group = "distribution"
    description = "Baut mit jlink eine Minimal-JRE nach build/dist/jre — von der Desktop-App gebündelt."

    val launcher = javaToolchains.launcherFor(java.toolchain).get()
    val jlink = launcher.metadata.installationPath.file("bin/jlink").asFile
    val output = runtimeDirectory.get().asFile

    inputs.property("modules", backendModules)
    inputs.property("java", launcher.metadata.javaRuntimeVersion)
    outputs.dir(runtimeDirectory)

    // jlink weigert sich, ein vorhandenes Verzeichnis zu überschreiben.
    doFirst { output.deleteRecursively() }

    commandLine(
        jlink.absolutePath,
        "--add-modules", backendModules.joinToString(","),
        "--strip-debug",
        "--no-header-files",
        "--no-man-pages",
        "--compress", "zip-6",
        // Das Basisarchiv mit den JDK-Klassen. Ohne es teilt die JRE keine einzige Klasse
        // vorbereitet aus — und das Anwendungsarchiv (siehe backend.ts) ließe sich gar nicht
        // erst erzeugen, denn es baut auf diesem auf.
        "--generate-cds-archive",
        "--output", output.absolutePath,
    )
}

/**
 * Packt das Fat-Jar in Anwendungs-Jar plus `lib/` aus.
 *
 * Spring Boots verschachtelte Jars kosten beim Start spürbar Zeit — jede Klasse kommt aus
 * einem Zip im Zip. Ausgepackt liest die JVM einen gewöhnlichen Klassenpfad, und nur so
 * lässt sich das CDS-Archiv erzeugen, mit dem die App startet (siehe `backend.ts`).
 */
val extractBackend by tasks.registering {
    group = "distribution"
    description = "Entpackt das Boot-Jar nach build/dist/backend (Anwendungs-Jar + lib/)."
    dependsOn(tasks.bootJar, jlinkRuntime)

    val bootJarFile = tasks.bootJar.flatMap { it.archiveFile }
    inputs.file(bootJarFile)
    outputs.dir(backendDirectory)

    doLast {
        val output = backendDirectory.get().asFile
        output.deleteRecursively()
        val java = runtimeDirectory.get().file("bin/java").asFile

        providers.exec {
            commandLine(
                java.absolutePath,
                "-Djarmode=tools",
                "-jar", bootJarFile.get().asFile.absolutePath,
                "extract",
                "--destination", output.absolutePath,
            )
        }.result.get().assertNormalExitValue()

        // Ohne Version im Namen: sonst müsste die electron-builder-Konfiguration die
        // Projektversion kennen und bei jedem Versionssprung nachgezogen werden.
        val extracted = output.listFiles { file -> file.extension == "jar" }?.singleOrNull()
            ?: throw GradleException("In $output steht nicht genau ein Anwendungs-Jar.")
        check(extracted.renameTo(output.resolve(BACKEND_JAR))) { "Konnte $extracted nicht umbenennen." }
    }
}

/**
 * Startet das ausgepackte Backend einmal unter der jlink-JRE und lässt es rechnen.
 *
 * Das ist die Probe auf [backendModules]: ein fehlendes Java-Modul fällt sonst erst in der
 * installierten App auf — dort aber ohne Konsole. Geprüft wird deshalb nicht nur, ob der
 * Server antwortet, sondern auch, ob eine Formel durchläuft. Symja holt sich unterwegs
 * Klassen aus Ecken des JDK, die kein Spring-Start berührt.
 */
val verifyRuntime by tasks.registering {
    group = "verification"
    description = "Prüft, ob das Backend unter der jlink-JRE startet und rechnet."
    dependsOn(extractBackend)

    inputs.dir(backendDirectory)
    outputs.file(layout.buildDirectory.file("tmp/verifyRuntime.log"))

    doLast {
        val backend = backendDirectory.get().asFile
        val java = runtimeDirectory.get().file("bin/java").asFile
        val port = 8438 // nicht der Default-Port: eine laufende App soll nicht stören
        val log = layout.buildDirectory.file("tmp/verifyRuntime.log").get().asFile
        log.parentFile.mkdirs()

        // Relativer Klassenpfad aus dem Backend-Verzeichnis heraus — genau so startet die
        // App das Backend auch (siehe `findBackend` in backend.ts).
        val process = ProcessBuilder(java.absolutePath, "-jar", BACKEND_JAR, "--server.port=$port")
            .directory(backend)
            .redirectErrorStream(true)
            .redirectOutput(log)
            .start()

        try {
            if (!awaitHealth(port, 90_000) { process.isAlive }) {
                throw GradleException(
                    "Das Backend ist unter der jlink-JRE nicht hochgekommen. Vermutlich fehlt ein " +
                        "Modul in backendModules — Details in ${log.absolutePath}",
                )
            }
            val result = evaluate(port, """F_1 = 2kg \cdot 10\frac{m}{s^2}""")
            if (!result.contains(""""unit":"N"""")) {
                throw GradleException(
                    "Das Backend rechnet unter der jlink-JRE nicht richtig. Vermutlich fehlt ein " +
                        "Modul in backendModules — es antwortete: $result",
                )
            }
            logger.lifecycle("Backend läuft und rechnet unter der jlink-JRE (${backendModules.size} Module).")
        } finally {
            process.destroy()
            if (!process.waitFor(10, TimeUnit.SECONDS)) process.destroyForcibly()
        }
    }
}

/** Wartet, bis `/api/health` auf [port] antwortet, solange [alive] gilt. */
fun awaitHealth(port: Int, timeoutMs: Long, alive: () -> Boolean): Boolean {
    val deadline = System.currentTimeMillis() + timeoutMs
    while (System.currentTimeMillis() < deadline && alive()) {
        val healthy = runCatching {
            URI("http://127.0.0.1:$port/api/health").toURL().openStream()
                .use { stream -> stream.readBytes().decodeToString() }
                .contains("\"status\":\"ok\"")
        }.getOrDefault(false)
        if (healthy) return true
        Thread.sleep(250)
    }
    return false
}

/** Lässt das Backend eine Formel rechnen und gibt die Antwort als JSON zurück. */
fun evaluate(port: Int, latex: String): String {
    val connection = URI("http://127.0.0.1:$port/api/evaluate").toURL().openConnection() as HttpURLConnection
    connection.requestMethod = "POST"
    connection.doOutput = true
    connection.setRequestProperty("Content-Type", "application/json")
    val body = """{"latex":"${latex.replace("\\", "\\\\")}","sessionId":"verify"}"""
    connection.outputStream.use { it.write(body.toByteArray()) }
    val stream = if (connection.responseCode < 400) connection.inputStream else connection.errorStream
    return stream.use { it.readBytes().decodeToString() }
}

/** Legt Backend und JRE unter `build/dist` so ab, wie die Desktop-App sie erwartet. */
val desktopResources by tasks.registering {
    group = "distribution"
    description = "Sammelt das ausgepackte Backend und die jlink-JRE unter build/dist für electron-builder."
    dependsOn(verifyRuntime)
}
