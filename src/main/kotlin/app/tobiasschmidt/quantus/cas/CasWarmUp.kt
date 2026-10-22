package app.tobiasschmidt.quantus.cas

import app.tobiasschmidt.quantus.session.Session
import app.tobiasschmidt.quantus.session.SessionCalculator
import org.slf4j.LoggerFactory
import org.springframework.boot.context.event.ApplicationReadyEvent
import org.springframework.context.ApplicationListener
import org.springframework.stereotype.Component

/**
 * Computes one throwaway formula right after startup so the user's first one does not have to.
 *
 * Symja builds its symbol table when it is first used — a good second of work that has
 * nothing to do with the formula that triggers it. Here it happens next to the idle window
 * instead of in front of the first result.
 *
 * The thread is a daemon and swallows what it catches: this is a head start, not a service.
 * Should it fail, the calculation it stands in for will fail just the same, and then with a
 * message the user can act on.
 */
@Component
class CasWarmUp(private val calculator: SessionCalculator) : ApplicationListener<ApplicationReadyEvent> {

    private val log = LoggerFactory.getLogger(javaClass)

    override fun onApplicationEvent(event: ApplicationReadyEvent) {
        val thread = Thread(::warmUp, "cas-warmup")
        thread.isDaemon = true
        thread.start()
    }

    private fun warmUp() {
        val started = System.nanoTime()
        runCatching { calculator.evaluate(Session(SESSION_ID), FORMULA) }
            .onSuccess { log.info("CAS ready ({} ms).", (System.nanoTime() - started) / 1_000_000) }
            .onFailure { failure -> log.debug("CAS warm-up failed, first calculation stays slow.", failure) }
    }

    private companion object {
        /** A definition with a unit — that walks through parser, CAS, units and serializer. */
        const val FORMULA = """F_1 = 2kg \cdot 10\frac{m}{s^2}"""

        /** Its own session, so the warm-up leaves no variable behind in a real one. */
        const val SESSION_ID = "cas-warmup"
    }
}
