package app.tobiasschmidt.quantus

import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test
import org.springframework.boot.test.context.runner.ApplicationContextRunner
import kotlin.test.assertFalse
import kotlin.test.assertTrue

/**
 * Der Watchdog ist absichtlich abschaltbar: nur die Desktop-App startet das Backend mit einer
 * stdin-Pipe. `bootRun` und der Docker-Betrieb (N4) laufen ohne Elternprozess, der auf sie
 * aufpasst — dort dürfte er nicht anspringen.
 *
 * Getestet wird nur die Registrierung. Der Lauf selbst liest `System.in` und würde bei EOF die
 * JVM beenden — im Testprozess wäre das die Test-JVM. Das Zusammenspiel mit Electron deckt
 * stattdessen der manuelle Start der App ab.
 */
class ParentProcessWatchdogTest {

    private val contexts = ApplicationContextRunner().withUserConfiguration(ParentProcessWatchdog::class.java)

    @Test
    @DisplayName("ohne Schalter gibt es keinen Watchdog")
    fun disabledByDefault() {
        contexts.run { context -> assertFalse(context.containsBean("parentProcessWatchdog")) }
    }

    @Test
    @DisplayName("mit quantus.parent-watch=true wird er registriert")
    fun enabledByProperty() {
        contexts.withPropertyValues("quantus.parent-watch=true")
            .run { context -> assertTrue(context.containsBean("parentProcessWatchdog")) }
    }
}
