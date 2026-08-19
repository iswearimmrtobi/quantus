package app.tobiasschmidt.quantus.latex

import java.math.BigDecimal
import javax.measure.Unit

/**
 * The parsed LaTeX input. Every node knows where it started in the source so errors can
 * point at the offending spot (F2: parse errors with a position).
 */
sealed interface Node {
    val position: Int
}

/** A literal number. Always a [BigDecimal] — never a double (see `exactExpr`). */
data class NumberNode(val value: BigDecimal, override val position: Int) : Node

/** A unit, either bare (`kg`) or explicitly marked up (`\mathrm{kg}`). */
data class UnitNode(val unit: Unit<*>, val symbol: String, override val position: Int) : Node

/** A reserved constant name: `\pi`, `e`, `i`. */
data class ConstantNode(val name: String, override val position: Int) : Node

/**
 * A variable already substituted with its session value (F6 step 2: "eingesetzt"). [value] is
 * rounded to display precision — evaluation never uses this node, it resolves the original
 * [VariableNode] against the session directly so the result stays exact.
 */
data class QuantityLiteralNode(val value: BigDecimal, val unit: Unit<*>, override val position: Int) : Node

/**
 * A variable. Per F2 a variable always carries a subscript — an identifier without one is a
 * unit or a constant, never a variable.
 */
data class VariableNode(val name: String, val index: String, override val position: Int) : Node

data class BinaryNode(
    val operator: Operator,
    val left: Node,
    val right: Node,
    override val position: Int,
) : Node

/** `\frac{a}{b}`, and what every `/` turns into during normalisation (F2, auto-fraction). */
data class FractionNode(val numerator: Node, val denominator: Node, override val position: Int) : Node

data class PowerNode(val base: Node, val exponent: Node, override val position: Int) : Node

/** `\sqrt{x}` or `\sqrt[n]{x}`. */
data class RootNode(val radicand: Node, val degree: Node?, override val position: Int) : Node

data class NegationNode(val operand: Node, override val position: Int) : Node

/**
 * An explicitly parenthesised sub-expression. Kept in the tree because explicit parentheses
 * win over the fraction-chain normalisation (`a/(b/c)` must not collapse); the serialiser
 * itself re-derives parentheses from operator precedence and treats this node as transparent.
 */
data class GroupNode(val inner: Node, override val position: Int) : Node

enum class Operator {
    PLUS,
    MINUS,
    TIMES,

    /** `/` — replaced by a [FractionNode] during normalisation. */
    DIVIDE,

    /** Juxtaposition: `2kg`, `10\frac{m}{s^2}`, `2\pi`. */
    IMPLICIT_TIMES,
}

/** The left-hand side of `F = …`. Phase 3 turns a label with a subscript into a variable definition. */
data class Label(val name: String, val index: String?, val position: Int)

/** A complete input: an optional `label =` and the expression itself. */
data class Formula(val label: Label?, val expression: Node, val source: String)
