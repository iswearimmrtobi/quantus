package app.tobiasschmidt.quantus.eval

import app.tobiasschmidt.quantus.cas.CasEngine
import app.tobiasschmidt.quantus.latex.BinaryNode
import app.tobiasschmidt.quantus.latex.Formula
import app.tobiasschmidt.quantus.latex.FractionNode
import app.tobiasschmidt.quantus.latex.GroupNode
import app.tobiasschmidt.quantus.latex.LatexParser
import app.tobiasschmidt.quantus.latex.LatexSerializer
import app.tobiasschmidt.quantus.latex.NegationNode
import app.tobiasschmidt.quantus.latex.Node
import app.tobiasschmidt.quantus.latex.PowerNode
import app.tobiasschmidt.quantus.latex.RootNode
import app.tobiasschmidt.quantus.latex.VariableNode
import app.tobiasschmidt.quantus.quantity.PhysicalQuantity
import app.tobiasschmidt.quantus.units.UnitRegistry
import java.math.BigDecimal

/**
 * LaTeX in, LaTeX out.
 *
 * `F = 2kg \cdot 10\frac{m}{s^2}` goes in, `F = 20\,\mathrm{N}` comes out — together with the
 * three-step presentation of F6: symbolic → substituted → result. The middle step only
 * appears when there is something to substitute, i.e. when the input uses session variables;
 * otherwise symbolic and substituted would be the same line twice.
 */
class LatexCalculator(private val precision: Int = CasEngine.DEFAULT_PRECISION) {

    @JvmOverloads
    fun evaluate(latex: String, variables: Map<String, PhysicalQuantity> = emptyMap()): Calculation {
        // The engine has to exist before any expression is built: it installs the
        // thread-local EvalEngine that PhysicalQuantity's arithmetic uses.
        val engine = CasEngine(precision)
        val formula = LatexParser(latex).parse()
        val quantity = ExpressionEvaluator(variables).evaluate(formula.expression).simplify()
        val value = quantity.decimal(engine)

        val result = LatexSerializer.quantity(value, quantity.unit)
        val prefix = formula.label?.let { "${LatexSerializer.label(it)} = " } ?: ""
        return Calculation(
            formula = formula,
            input = LatexSerializer.formula(formula),
            result = result,
            latex = prefix + result,
            steps = steps(formula, prefix, result, variables, engine),
            value = value,
            unit = UnitRegistry.format(quantity.unit),
            quantityName = quantity.quantityName,
            quantity = quantity,
        )
    }

    private fun steps(
        formula: Formula,
        prefix: String,
        result: String,
        variables: Map<String, PhysicalQuantity>,
        engine: CasEngine,
    ): List<Step> {
        val symbolic = Step(StepKind.SYMBOLIC, prefix + LatexSerializer.expression(formula.expression))
        // A definition such as `m_1 = 2kg` has nothing to show but itself.
        if (symbolic.latex == prefix + result) return listOf(Step(StepKind.RESULT, symbolic.latex))

        val steps = mutableListOf(symbolic)
        if (usesVariables(formula.expression)) {
            val substituted = Substitution(engine).substitute(formula.expression, variables)
            steps += Step(StepKind.SUBSTITUTED, prefix + LatexSerializer.expression(substituted))
        }
        steps += Step(StepKind.RESULT, prefix + result)
        return steps
    }

    private fun usesVariables(node: Node): Boolean =
        node is VariableNode || children(node).any { usesVariables(it) }

    private fun children(node: Node): List<Node> = when (node) {
        is GroupNode -> listOf(node.inner)
        is NegationNode -> listOf(node.operand)
        is FractionNode -> listOf(node.numerator, node.denominator)
        is PowerNode -> listOf(node.base, node.exponent)
        is RootNode -> listOfNotNull(node.radicand, node.degree)
        is BinaryNode -> listOf(node.left, node.right)
        else -> emptyList()
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
    val steps: List<Step>,
    val value: BigDecimal,
    val unit: String,
    val quantityName: String?,
    val quantity: PhysicalQuantity,
) {

    /** The whole calculation as one copyable `align*` block (F7). */
    val alignBlock: String
        get() = steps.joinToString(
            separator = " \\\\\n",
            prefix = "\\begin{align*}\n",
            postfix = "\n\\end{align*}",
        ) { "  ${it.latex}" }
}

/** One line of the presentation from F6. */
data class Step(val kind: StepKind, val latex: String)

enum class StepKind {
    /** The formula as it was typed: `F = m_1 \cdot a_1`. */
    SYMBOLIC,

    /** The same formula with the session's values put in. */
    SUBSTITUTED,

    /** The evaluated result. */
    RESULT,
}
