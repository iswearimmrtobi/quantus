package app.tobiasschmidt.quantus.session

import org.springframework.stereotype.Component
import java.util.concurrent.ConcurrentHashMap

/**
 * All live sessions, keyed by the id the client sends.
 *
 * The desktop app runs a single local session; the id exists so the same backend can serve
 * several clients when it is self-hosted (N4). A request without an id gets the default one.
 */
@Component
class SessionStore {

    private val sessions = ConcurrentHashMap<String, Session>()

    fun session(id: String?): Session {
        val key = id?.takeIf { it.isNotBlank() } ?: DEFAULT_SESSION
        return sessions.computeIfAbsent(key) { Session(it) }
    }

    companion object {
        const val DEFAULT_SESSION = "default"
    }
}
