package app.tobiasschmidt.quantus

import org.slf4j.LoggerFactory
import org.springframework.boot.ExitCodeGenerator
import org.springframework.boot.SpringApplication
import org.springframework.boot.autoconfigure.condition.ConditionalOnProperty
import org.springframework.boot.context.event.ApplicationReadyEvent
import org.springframework.context.ApplicationListener
import org.springframework.context.ConfigurableApplicationContext
import org.springframework.stereotype.Component
import kotlin.system.exitProcess

/**
 * Beendet das Backend, wenn der Elternprozess verschwindet — eingeschaltet über
 * `--quantus.parent-watch=true`, das die Desktop-App beim Start mitgibt.
 *
 * Normalerweise beendet Electron den Kindprozess selbst. Wird die App aber hart abgeschossen
 * (SIGKILL, Absturz, Kill des AppImage-Wrappers), kommt sie nicht mehr dazu — und ein
 * verwaistes Backend bliebe auf Port 8437 zurück, genau das Zombie-Risiko aus dem Spec.
 *
 * Der Anker ist stdin: Electron startet den Prozess mit einer Pipe daran und schreibt nie
 * hinein. Stirbt der Elternprozess, schließt das Betriebssystem die Pipe, der Lesevorgang
 * liefert EOF — und das ist das Signal zum Aufräumen. Ohne den Schalter rührt niemand stdin
 * an, der Docker- und der `bootRun`-Betrieb bleiben also unberührt.
 */
@Component
@ConditionalOnProperty("quantus.parent-watch", havingValue = "true")
class ParentProcessWatchdog(
    private val context: ConfigurableApplicationContext,
) : ApplicationListener<ApplicationReadyEvent> {

    private val log = LoggerFactory.getLogger(javaClass)

    override fun onApplicationEvent(event: ApplicationReadyEvent) {
        val watcher = Thread(::awaitParentExit, "parent-watchdog")
        watcher.isDaemon = true
        watcher.start()
    }

    private fun awaitParentExit() {
        val reason = runCatching {
            // Blockiert, bis der Elternprozess schreibt (tut er nie) oder die Pipe zugeht.
            if (System.`in`.read() == -1) "parent process closed stdin" else "shutdown requested via stdin"
        }.getOrElse { failure -> "stdin is gone (${failure.javaClass.simpleName})" }

        log.info("Shutting down: {}.", reason)
        val status = runCatching { SpringApplication.exit(context, ExitCodeGenerator { 0 }) }.getOrDefault(0)
        // Notausgang, falls ein Nicht-Daemon-Thread den regulären Shutdown blockiert.
        exitProcess(status)
    }
}
