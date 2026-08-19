package app.tobiasschmidt.quantus.eval

import app.tobiasschmidt.quantus.QuantusException
import app.tobiasschmidt.quantus.cas.Constants
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

/**
 * Evaluates a parsed AST into a [PhysicalQuantity] — the step where the LaTeX side hands
 * over to the Symja/Indriya side.
 *
 * Values stay exact throughout: a number literal enters as a rational, π stays π. Only the
 * caller turns the result into decimal digits.
 *
 * [resolveVariable] looks a variable up in the session (phase 3, F5); the default resolver
 * finds nothing, so a bare `ExpressionEvaluator()` still rejects every variable as before.
 */
class ExpressionEvaluator(private val resolveVariable: (VariableKey) -> PhysicalQuantity? = { null }) {

    fun evaluate(node: Node): PhysicalQuantity = when (node) {
        is NumberNode -> PhysicalQuantity.dimensionless(node.value)
        is UnitNode -> PhysicalQuantity.of(1, node.unit)
        is ConstantNode -> PhysicalQuantity.dimensionless(
            Constants.find(node.name) ?: throw QuantusException("Unknown constant '${node.name}'."),
        )

        is VariableNode -> resolveVariable(VariableKey(node.name, node.index)) ?: throw UnresolvedVariableException(
            "The variable ${node.name}_${node.index} has no value yet. Define it first, e.g. ${node.name}_${node.index} = 5\\,\\mathrm{m}.",
        )

        is GroupNode -> evaluate(node.inner)
        is NegationNode -> -evaluate(node.operand)
        is FractionNode -> evaluate(node.numerator) / evaluate(node.denominator)
        is PowerNode -> evaluate(node.base).pow(evaluate(node.exponent))
        is RootNode -> evaluate(node.radicand).root(degreeOf(node))
        is BinaryNode -> binary(node)
    }

    private fun binary(node: BinaryNode): PhysicalQuantity {
        val left = evaluate(node.left)
        val right = evaluate(node.right)
        return when (node.operator) {
            Operator.PLUS -> left + right
            Operator.MINUS -> left - right
            Operator.TIMES, Operator.IMPLICIT_TIMES -> left * right
            Operator.DIVIDE -> left / right
        }
    }

    private fun degreeOf(node: RootNode): Int {
        val degree = node.degree ?: return 2
        val quantity = evaluate(degree)
        if (!quantity.isDimensionless) {
            throw QuantusException("The degree of a root must be a plain number, not a quantity with a unit.")
        }
        val value = quantity.value.toIntDefault()
        if (value <= 0) {
            throw QuantusException("The degree of a root must be a positive whole number, but was '${quantity.value}'.")
        }
        return value
    }
}

/** A variable was used before it was defined in the session (F5). */
class UnresolvedVariableException(message: String) : QuantusException(message)
