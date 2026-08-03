import java.net.URI
import java.util.concurrent.TimeUnit

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

val runtimeDirectory = layout.buildDirectory.dir("jre")

/** Die Minimal-JRE für den mitgelieferten Backend-Prozess (Phase 6, Packaging). */
val jlinkRuntime by tasks.registering(Exec::class) {
    group = "distribution"
    description = "Baut mit jlink eine Minimal-JRE nach build/jre — von der Desktop-App gebündelt."

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
        "--output", output.absolutePath,
    )
}

/**
 * Startet das Fat-Jar einmal mit der gebauten Minimal-JRE und wartet auf `/api/health`.
 *
 * Ein fehlendes Modul fällt sonst erst in der installierten App auf — dort aber ohne Konsole.
 */
val verifyRuntime by tasks.registering {
    group = "verification"
    description = "Prüft, ob das Backend unter der jlink-JRE startet und antwortet."
    dependsOn(jlinkRuntime, tasks.bootJar)

    doLast {
        val javaBinary = runtimeDirectory.get().file("bin/java").asFile
        val jar = tasks.bootJar.get().archiveFile.get().asFile
        val port = 8438 // nicht der Default-Port: eine laufende App soll nicht stören
        val log = layout.buildDirectory.file("tmp/verifyRuntime.log").get().asFile
        log.parentFile.mkdirs()

        val process = ProcessBuilder(javaBinary.absolutePath, "-jar", jar.absolutePath, "--server.port=$port")
            .redirectErrorStream(true)
            .redirectOutput(log)
            .start()
        try {
            val deadline = System.currentTimeMillis() + 90_000
            var healthy = false
            while (System.currentTimeMillis() < deadline && process.isAlive && !healthy) {
                healthy = runCatching {
                    val body = URI("http://127.0.0.1:$port/api/health").toURL().openStream()
                        .use { stream -> stream.readBytes().decodeToString() }
                    body.contains("\"status\":\"ok\"")
                }.getOrDefault(false)
                if (!healthy) Thread.sleep(500)
            }
            if (!healthy) {
                throw GradleException(
                    "Das Backend ist unter der jlink-JRE nicht hochgekommen. Vermutlich fehlt ein " +
                        "Modul in backendModules — Details in ${log.absolutePath}",
                )
            }
            logger.lifecycle("Backend läuft unter der jlink-JRE (${backendModules.size} Module).")
        } finally {
            process.destroy()
            if (!process.waitFor(10, TimeUnit.SECONDS)) process.destroyForcibly()
        }
    }
}

/**
 * Legt Jar und JRE unter `build/dist` so ab, wie die Desktop-App sie erwartet.
 *
 * Das Jar heißt hier `quantus.jar` ohne Version: sonst müsste die electron-builder-Konfiguration
 * die Projektversion kennen und bei jedem Versionssprung nachgezogen werden.
 */
val desktopResources by tasks.registering(Copy::class) {
    group = "distribution"
    description = "Sammelt Backend-Jar und jlink-JRE unter build/dist für electron-builder."
    dependsOn(verifyRuntime)

    val staging = layout.buildDirectory.dir("dist")

    // jlink legt seine Lizenztexte schreibgeschützt ab; ein zweiter Lauf käme sonst beim
    // Überschreiben nicht an ihnen vorbei.
    doFirst { staging.get().asFile.deleteRecursively() }

    into(staging)
    from(tasks.bootJar) {
        into("backend")
        rename { "quantus.jar" }
    }
    from(runtimeDirectory) { into("jre") }
}
