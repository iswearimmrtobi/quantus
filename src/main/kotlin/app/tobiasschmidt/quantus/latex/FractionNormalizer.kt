package app.tobiasschmidt.quantus.latex

/**
 * F2 — the auto-fraction rewrite: every `/` becomes a `\frac{}{}`.
 *
 * Operand boundaries follow standard precedence (`\cdot` and `/` rank equally and associate
 * to the left), so `a \cdot b / c` yields `\frac{a \cdot b}{c}`. Chains are flattened into a
 * common denominator rather than nested — `a/b/c` becomes `\frac{a}{b \cdot c}`, not
 * `\frac{\frac{a}{b}}{c}`. Explicit parentheses always win: `a/(b/c)` keeps its inner
 * fraction, because a parenthesised sub-expression is a [GroupNode] and ends the chain.
 *
 * This runs as a normalisation pass right after parsing, so evaluation and serialisation
 * both see the same tree.
 */
object FractionNormalizer {

    fun normalize(node: Node): Node = when (node) {
        is BinaryNode -> normalizeBinary(node)
        is FractionNode -> FractionNode(normalize(node.numerator), normalize(node.denominator), node.position)
        is PowerNode -> PowerNode(normalize(node.base), normalize(node.exponent), node.position)
        is RootNode -> RootNode(normalize(node.radicand), node.degree?.let { normalize(it) }, node.position)
        is NegationNode -> NegationNode(normalize(node.operand), node.position)
        is GroupNode -> GroupNode(normalize(node.inner), node.position)
        is FunctionCallNode -> FunctionCallNode(node.name, normalize(node.argument), node.position)
        else -> node
    }

    private fun normalizeBinary(node: BinaryNode): Node {
        if (node.operator != Operator.DIVIDE) {
            return BinaryNode(node.operator, normalize(node.left), normalize(node.right), node.position)
        }

        val denominators = ArrayDeque<Node>()
        var numerator: Node = node
        while (numerator is BinaryNode && numerator.operator == Operator.DIVIDE) {
            denominators.addFirst(numerator.right)
            numerator = numerator.left
        }

        val denominator = denominators
            .map { normalize(it) }
            .reduce { left, right -> BinaryNode(Operator.TIMES, left, right, left.position) }
        return FractionNode(normalize(numerator), denominator, node.position)
    }
}
