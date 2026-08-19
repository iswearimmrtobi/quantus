package app.tobiasschmidt.quantus.cas

import org.matheclipse.core.form.tex.TeXFormFactory
import org.matheclipse.core.interfaces.IExpr

/**
 * Symja's own TeX exporter, for the pure-math sub-trees the hand-written `LatexSerializer`
 * does not cover: derivatives and integrals (F8). Phase 0's spike 3 found it unfit for F6/F7's
 * step presentation in general (no unit markup, no German decimal comma) but clean for exactly
 * this — a fully evaluated expression built only from numbers, symbols and functions like
 * `Sin`/`Cos`/`Log`, with no units and no `HoldForm` involved.
 */
object SymjaLatex {
    fun format(expr: IExpr): String {
        val builder = StringBuilder()
        TeXFormFactory().convert(builder, expr)
        return builder.toString().trim()
    }
}
