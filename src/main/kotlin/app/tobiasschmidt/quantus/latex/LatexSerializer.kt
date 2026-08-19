package app.tobiasschmidt.quantus.latex

import tech.units.indriya.AbstractUnit
import java.math.BigDecimal
import javax.measure.Unit

/**
 * AST and results back to LaTeX (F7).
 *
 * Parentheses are not taken from the input but re-derived from operator precedence, so the
 * output is canonical: redundant brackets disappear, necessary ones stay. Units are always
 * marked up as `\mathrm{}`, and a number followed by a pure unit is joined with a thin space
 * (`2\,\mathrm{kg}`) instead of a `\cdot`.
 */
object LatexSerializer {

    /** The full input including its label: `F = 2\,\mathrm{kg} \cdot 10\,\frac{\mathrm{m}}{\mathrm{s}^{2}}`. */
    fun formula(formula: Formula): String {
        val expression = expression(formula.expression)
        return formula.label?.let { "${label(it)} = $expression" } ?: expression
    }

    fun label(label: Label): String = if (label.index == null) label.name else "${label.name}_{${label.index}}"

    fun expression(node: Node): String = render(node, PRECEDENCE_ADDITIVE)

    /** A result: value and unit, e.g. `20\,\mathrm{N}`. */
    fun quantity(value: BigDecimal, unit: Unit<*>): String = withUnit(LatexNumbers.format(value), unit)

    /** F4 — a complex result in cartesian form, plus unit: `(11 - 2\,i)\,\mathrm{N}`. */
    fun complexQuantity(re: BigDecimal, im: BigDecimal, unit: Unit<*>): String =
        withUnit(LatexComplex.cartesian(re, im), unit)

    /** F4 — a complex result in polar form, plus unit. */
    fun polarQuantity(r: BigDecimal, phi: BigDecimal, unit: Unit<*>): String =
        withUnit(LatexComplex.polar(r, phi), unit)

    private fun withUnit(number: String, unit: Unit<*>): String {
        if (unit == AbstractUnit.ONE) return number
        return "$number\\,${UnitLatex.format(unit)}"
    }

    // ---- rendering --------------------------------------------------------------------

    private const val PRECEDENCE_ADDITIVE = 1
    private const val PRECEDENCE_MULTIPLICATIVE = 2
    private const val PRECEDENCE_POWER = 3
    private const val PRECEDENCE_ATOM = 4

    private fun render(node: Node, minPrecedence: Int): String {
        if (node is GroupNode) return render(node.inner, minPrecedence)
        val text = text(node)
        return if (precedence(node) < minPrecedence) "($text)" else text
    }

    private fun text(node: Node): String = when (node) {
        is NumberNode -> LatexNumbers.format(node.value)
        is UnitNode -> UnitLatex.symbol(node.symbol)
        is ConstantNode -> node.name
        is VariableNode -> "${node.name}_{${node.index}}"
        is MathSymbolNode -> node.name
        is FunctionCallNode -> "\\${node.name}\\left(${expression(node.argument)}\\right)"
        is QuantityLiteralNode -> quantity(node.value, node.unit)
        is NegationNode -> "-${render(node.operand, PRECEDENCE_MULTIPLICATIVE)}"
        is FractionNode -> "\\frac{${expression(node.numerator)}}{${expression(node.denominator)}}"
        is RootNode -> root(node)
        is PowerNode -> power(node)
        is BinaryNode -> binary(node)
        is GroupNode -> expression(node.inner)
    }

    private fun root(node: RootNode): String {
        val radicand = expression(node.radicand)
        return node.degree?.let { "\\sqrt[${expression(it)}]{$radicand}" } ?: "\\sqrt{$radicand}"
    }

    private fun power(node: PowerNode): String {
        // A fraction (or a substituted "value unit") as the base needs brackets of its own —
        // \frac{a}{b}^{2} and 0{,}5\,\mathrm{m}^{2} both read wrong.
        val base = if (node.base is FractionNode || node.base is RootNode || node.base is QuantityLiteralNode) {
            "(${text(node.base)})"
        } else {
            render(node.base, PRECEDENCE_ATOM)
        }
        return "$base^{${expression(node.exponent)}}"
    }

    private fun binary(node: BinaryNode): String = when (node.operator) {
        Operator.PLUS -> "${render(node.left, PRECEDENCE_ADDITIVE)} + ${render(node.right, PRECEDENCE_ADDITIVE)}"
        // The right operand of a minus needs brackets around a sum: a - (b - c).
        Operator.MINUS -> "${render(node.left, PRECEDENCE_ADDITIVE)} - ${render(node.right, PRECEDENCE_MULTIPLICATIVE)}"
        Operator.TIMES -> multiplication(node, " \\cdot ")
        // Normalisation turns every division into a fraction; this is only a safety net.
        Operator.DIVIDE -> "\\frac{${expression(node.left)}}{${expression(node.right)}}"
        Operator.IMPLICIT_TIMES -> multiplication(node, if (isUnitOnly(node.right)) "\\," else " \\cdot ")
    }

    private fun multiplication(node: BinaryNode, separator: String): String {
        // A leading minus needs no brackets: -5\,\mathrm{m} reads back as -(5 m), same value.
        val left = if (node.left is NegationNode) text(node.left) else render(node.left, PRECEDENCE_MULTIPLICATIVE)
        return left + separator + render(node.right, PRECEDENCE_MULTIPLICATIVE)
    }

    private fun precedence(node: Node): Int = when (node) {
        is BinaryNode -> when (node.operator) {
            Operator.PLUS, Operator.MINUS -> PRECEDENCE_ADDITIVE
            else -> PRECEDENCE_MULTIPLICATIVE
        }

        is NegationNode -> PRECEDENCE_ADDITIVE
        is PowerNode -> PRECEDENCE_POWER
        is GroupNode -> precedence(node.inner)
        else -> PRECEDENCE_ATOM
    }

    /**
     * Is this sub-tree nothing but units? Then the product before it is the "number times
     * unit" of physics notation and gets a thin space: `10\,\frac{\mathrm{m}}{\mathrm{s}^{2}}`.
     */
    private fun isUnitOnly(node: Node): Boolean = when (node) {
        is UnitNode -> true
        is PowerNode -> isUnitOnly(node.base)
        is FractionNode -> isUnitOnly(node.numerator) && isUnitOnly(node.denominator)
        is GroupNode -> isUnitOnly(node.inner)
        is BinaryNode -> node.operator != Operator.PLUS && node.operator != Operator.MINUS &&
            isUnitOnly(node.left) && isUnitOnly(node.right)

        else -> false
    }
}
