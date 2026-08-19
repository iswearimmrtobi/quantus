plugins {
    application
    alias(libs.plugins.kotlin.jvm)
    alias(libs.plugins.javafx)
}

group = "app.tobiasschmidt"
version = "0.1.0-SNAPSHOT"

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

application {
    mainClass = "app.tobiasschmidt.quantus.QuantusApplicationKt"
}

javafx {
    version = libs.versions.javafx.get()
    // controls: native panels (result, steps, variables, history); web: WebView for the
    // embedded MathLive editor (see spec-si-rechner-v3.md Abschnitt 4).
    modules = listOf("javafx.controls", "javafx.web")
}

repositories {
    mavenCentral()
}

dependencies {
    // CAS: symbolische und numerische Auswertung, beliebige Präzision
    implementation(libs.symja.core)
    // Einheiten: JSR-385-Referenzimplementierung, Dimensionsprüfung
    implementation(libs.indriya)

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
