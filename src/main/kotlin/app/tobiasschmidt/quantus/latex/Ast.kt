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
 * A variable. Per F2 a variable always carries a subscript — an identifier without one is a
 * unit or a constant, never a variable.
 */
data class VariableNode(val name: String, val index: String, override val position: Int) : Node {

    /** How the session addresses this variable: `v_1`. */
    val key: String get() = "${name}_$index"
}

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

/**
 * The left-hand side of `F = …`. A label **with** a subscript is a variable definition
 * (F5) — `r_1 = 0{,}5\,\mathrm{m}` defines `r_1`; a label without one is just a name for
 * the line, per the uniqueness rule of F2.
 */
data class Label(val name: String, val index: String?, val position: Int) {

    /** The session key of the variable this label defines, or `null` if it defines none. */
    val key: String? get() = index?.let { "${name}_$it" }
}

/** A complete input: an optional `label =` and the expression itself. */
data class Formula(val label: Label?, val expression: Node, val source: String)
