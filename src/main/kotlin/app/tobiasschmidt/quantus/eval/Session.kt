package app.tobiasschmidt.quantus.eval

import app.tobiasschmidt.quantus.quantity.PhysicalQuantity
import java.util.UUID

/** Identifies one calculation session; a fresh id starts with an empty variable set. */
@JvmInline
value class SessionId(val value: String) {
    companion object {
        fun random(): SessionId = SessionId(UUID.randomUUID().toString())
    }
}

/** A variable's name and subscript together (F5 uniqueness rule) — `r_1`, `\omega_0`. */
data class VariableKey(val name: String, val index: String) {
    override fun toString(): String = "${name}_$index"
}

data class VariableDefinition(val key: VariableKey, val value: PhysicalQuantity)

/**
 * One session's variable bindings (F5). Values stay exact — the same invariant as everywhere
 * else in [PhysicalQuantity] arithmetic — so a session outlives any single `CasEngine`
 * instance without losing precision; only display rounds to a given precision.
 */
class Session(val id: SessionId) {
    private val variables = linkedMapOf<VariableKey, PhysicalQuantity>()

    fun define(key: VariableKey, value: PhysicalQuantity) {
        variables[key] = value
    }

    fun resolve(key: VariableKey): PhysicalQuantity? = variables[key]

    fun variables(): List<VariableDefinition> = variables.map { (key, value) -> VariableDefinition(key, value) }
}

/** Sessions live only for the process lifetime — persistence is phase 6 (history). */
class SessionStore {
    private val sessions = mutableMapOf<SessionId, Session>()

    fun session(id: SessionId): Session = sessions.getOrPut(id) { Session(id) }
}
