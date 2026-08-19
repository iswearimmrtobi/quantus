package app.tobiasschmidt.quantus.eval

import app.tobiasschmidt.quantus.cas.CasEngine
import app.tobiasschmidt.quantus.latex.BinaryNode
import app.tobiasschmidt.quantus.latex.FractionNode
import app.tobiasschmidt.quantus.latex.GroupNode
import app.tobiasschmidt.quantus.latex.NegationNode
import app.tobiasschmidt.quantus.latex.Node
import app.tobiasschmidt.quantus.latex.PowerNode
import app.tobiasschmidt.quantus.latex.QuantityLiteralNode
import app.tobiasschmidt.quantus.latex.RootNode
import app.tobiasschmidt.quantus.latex.VariableNode

/**
 * F6 step 2 ("eingesetzt"): rewrites every [VariableNode] in the tree into a
 * [QuantityLiteralNode] carrying its session value, rounded to [engine]'s precision.
 */
object VariableSubstitution {

    /** `null` if the tree has no variable at all — callers use that to skip a step that
     * would render identically to the symbolic one. */
    fun substitute(node: Node, session: Session, engine: CasEngine): Node? {
        if (!containsVariable(node)) return null
        return replace(node, session, engine)
    }

    private fun containsVariable(node: Node): Boolean = when (node) {
        is VariableNode -> true
        is BinaryNode -> containsVariable(node.left) || containsVariable(node.right)
        is FractionNode -> containsVariable(node.numerator) || containsVariable(node.denominator)
        is PowerNode -> containsVariable(node.base) || containsVariable(node.exponent)
        is RootNode -> containsVariable(node.radicand) || (node.degree?.let(::containsVariable) ?: false)
        is NegationNode -> containsVariable(node.operand)
        is GroupNode -> containsVariable(node.inner)
        else -> false
    }

    private fun replace(node: Node, session: Session, engine: CasEngine): Node = when (node) {
        is VariableNode -> {
            val key = VariableKey(node.name, node.index)
            val quantity = session.resolve(key) ?: throw UnresolvedVariableException(
                "The variable ${node.name}_${node.index} has no value yet. Define it first, e.g. " +
                    "${node.name}_${node.index} = 5\\,\\mathrm{m}.",
            )
            QuantityLiteralNode(quantity.decimal(engine), quantity.unit, node.position)
        }

        is BinaryNode -> BinaryNode(
            node.operator,
            replace(node.left, session, engine),
            replace(node.right, session, engine),
            node.position,
        )

        is FractionNode -> FractionNode(
            replace(node.numerator, session, engine),
            replace(node.denominator, session, engine),
            node.position,
        )

        is PowerNode -> PowerNode(replace(node.base, session, engine), replace(node.exponent, session, engine), node.position)
        is RootNode -> RootNode(
            replace(node.radicand, session, engine),
            node.degree?.let { replace(it, session, engine) },
            node.position,
        )

        is NegationNode -> NegationNode(replace(node.operand, session, engine), node.position)
        is GroupNode -> GroupNode(replace(node.inner, session, engine), node.position)
        else -> node
    }
}
