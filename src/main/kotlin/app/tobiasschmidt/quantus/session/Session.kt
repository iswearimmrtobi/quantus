package app.tobiasschmidt.quantus.session

import app.tobiasschmidt.quantus.quantity.PhysicalQuantity
import java.math.BigDecimal

/**
 * A variable of the current session (F5): `r_1 = 0{,}5\,\mathrm{m}`.
 *
 * [quantity] is the exact value used for arithmetic; [value] and [latex] are its display
 * forms, computed once when the variable is defined.
 */
data class Variable(
    /** How the session addresses it: `r_1`. */
    val key: String,
    /** The name in LaTeX: `r_{1}`. */
    val symbol: String,
    /** Value and unit in LaTeX: `0{,}5\,\mathrm{m}`. */
    val latex: String,
    val value: BigDecimal,
    val unit: String,
    val quantity: PhysicalQuantity,
)

/**
 * One user session: the variables defined so far, in definition order.
 *
 * Sessions live in memory only — the Electron app owns exactly one and re-creates it on every
 * start. Persistence across restarts is phase 6, together with the history.
 */
class Session(val id: String) {

    private val definitions = LinkedHashMap<String, Variable>()

    @Synchronized
    fun variables(): List<Variable> = definitions.values.toList()

    /** The values as [PhysicalQuantity] — what the evaluator needs. */
    @Synchronized
    fun quantities(): Map<String, PhysicalQuantity> = definitions.mapValues { it.value.quantity }

    /** Defines a variable, replacing an earlier definition of the same name in place. */
    @Synchronized
    fun define(variable: Variable) {
        definitions[variable.key] = variable
    }

    @Synchronized
    fun remove(key: String): Boolean = definitions.remove(key) != null

    @Synchronized
    fun clear() = definitions.clear()
}
