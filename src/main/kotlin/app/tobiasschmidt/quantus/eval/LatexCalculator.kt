package app.tobiasschmidt.quantus.eval

import app.tobiasschmidt.quantus.cas.CasEngine
import app.tobiasschmidt.quantus.latex.Formula
import app.tobiasschmidt.quantus.latex.LatexParser
import app.tobiasschmidt.quantus.latex.LatexSerializer
import app.tobiasschmidt.quantus.quantity.PhysicalQuantity
import app.tobiasschmidt.quantus.units.UnitRegistry
import java.math.BigDecimal

/**
 * LaTeX in, LaTeX out — the round trip of phase 2.
 *
 * `F = 2kg \cdot 10\frac{m}{s^2}` goes in, `F = 20\,\mathrm{N}` comes out.
 *
 * Phase 3 grows the three-step presentation (symbolic → substituted → result) and the REST
 * endpoint on top of this; the pieces it needs — the parsed [Formula] and the exact
 * [PhysicalQuantity] — are part of the result already.
 */
class LatexCalculator(private val precision: Int = CasEngine.DEFAULT_PRECISION) {

    fun evaluate(latex: String): Calculation {
        // The engine has to exist before any expression is built: it installs the
        // thread-local EvalEngine that PhysicalQuantity's arithmetic uses.
        val engine = CasEngine(precision)
        val formula = LatexParser(latex).parse()
        val quantity = ExpressionEvaluator().evaluate(formula.expression).simplify()
        val value = quantity.decimal(engine)

        val result = LatexSerializer.quantity(value, quantity.unit)
        return Calculation(
            formula = formula,
            input = LatexSerializer.formula(formula),
            result = result,
            latex = formula.label?.let { "${LatexSerializer.label(it)} = $result" } ?: result,
            value = value,
            unit = UnitRegistry.format(quantity.unit),
            quantityName = quantity.quantityName,
            quantity = quantity,
        )
    }
}

/**
 * One evaluated input. [input] is the canonical re-rendering of what was typed (with `/`
 * turned into `\frac{}{}`), [latex] the finished line including the label.
 */
data class Calculation(
    val formula: Formula,
    val input: String,
    val result: String,
    val latex: String,
    val value: BigDecimal,
    val unit: String,
    val quantityName: String?,
    val quantity: PhysicalQuantity,
)
