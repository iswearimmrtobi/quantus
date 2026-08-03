package app.tobiasschmidt.quantus.cas

import org.matheclipse.core.expression.F
import org.matheclipse.core.interfaces.IExpr

/**
 * Reserved constant names (F2). They are looked up *after* the unit table, so a symbol that
 * is a unit stays a unit; the names here are chosen not to collide with one.
 *
 * The values are symbolic — π stays π until the very last numeric evaluation.
 * Phase 4 and the backlog add the physical constants (c, h, G, …) here.
 */
object Constants {

    private val values: Map<String, IExpr> = mapOf(
        "\\pi" to F.Pi,
        "e" to F.E,
        "i" to F.CI,
    )

    fun find(name: String): IExpr? = values[name]

    fun isConstant(name: String): Boolean = name in values
}
