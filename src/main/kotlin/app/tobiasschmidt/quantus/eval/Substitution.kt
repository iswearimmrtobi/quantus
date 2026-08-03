package app.tobiasschmidt.quantus.eval

import app.tobiasschmidt.quantus.cas.CasEngine
import app.tobiasschmidt.quantus.latex.BinaryNode
import app.tobiasschmidt.quantus.latex.ConstantNode
import app.tobiasschmidt.quantus.latex.FractionNode
import app.tobiasschmidt.quantus.latex.GroupNode
import app.tobiasschmidt.quantus.latex.NegationNode
import app.tobiasschmidt.quantus.latex.Node
import app.tobiasschmidt.quantus.latex.NumberNode
import app.tobiasschmidt.quantus.latex.Operator
import app.tobiasschmidt.quantus.latex.PowerNode
import app.tobiasschmidt.quantus.latex.RootNode
import app.tobiasschmidt.quantus.latex.UnitNode
import app.tobiasschmidt.quantus.latex.VariableNode
import app.tobiasschmidt.quantus.quantity.PhysicalQuantity
import app.tobiasschmidt.quantus.units.UnitRegistry
import tech.units.indriya.AbstractUnit

/**
 * Replaces variables in an AST by their values — the middle of the three steps from F6:
 * `F = m_1 \cdot a_1` becomes `F = 2\,\mathrm{kg} \cdot 9{,}81\,\frac{\mathrm{m}}{\mathrm{s}^{2}}`.
 *
 * The substituted tree is a display artefact only; the actual arithmetic runs on the exact
 * values in [ExpressionEvaluator], never on these decimals.
 */
class Substitution(private val engine: CasEngine) {

    fun substitute(node: Node, variables: Map<String, PhysicalQuantity>): Node = when (node) {
        is VariableNode -> variables[node.key]?.let { valueNode(it, node.position) } ?: node
        is GroupNode -> GroupNode(substitute(node.inner, variables), node.position)
        is NegationNode -> NegationNode(substitute(node.operand, variables), node.position)
        is FractionNode -> FractionNode(
            substitute(node.numerator, variables),
            substitute(node.denominator, variables),
            node.position,
        )

        is PowerNode -> PowerNode(
            substitute(node.base, variables),
            substitute(node.exponent, variables),
            node.position,
        )

        is RootNode -> RootNode(
            substitute(node.radicand, variables),
            node.degree?.let { substitute(it, variables) },
            node.position,
        )

        is BinaryNode -> BinaryNode(
            node.operator,
            substitute(node.left, variables),
            substitute(node.right, variables),
            node.position,
        )

        is NumberNode, is UnitNode, is ConstantNode -> node
    }

    /**
     * A quantity as an AST: number, and — unless it is dimensionless — its unit next to it.
     * Values without a decimal form (a complex intermediate result, say) yield `null`; the
     * variable then stays symbolic in the substituted step instead of the step being lost.
     */
    private fun valueNode(quantity: PhysicalQuantity, position: Int): Node? {
        val decimal = runCatching { quantity.decimal(engine) }.getOrNull() ?: return null
        val number = NumberNode(decimal, position)
        if (quantity.unit == AbstractUnit.ONE) return number
        val unit = UnitNode(quantity.unit, UnitRegistry.format(quantity.unit), position)
        return BinaryNode(Operator.IMPLICIT_TIMES, number, unit, position)
    }
}
