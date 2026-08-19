package app.tobiasschmidt.quantus

import org.junit.jupiter.api.Test

/** Meilenstein Phase 0: Symja und Indriya laufen im selben Classpath, keine Kollision. */
class QuantusApplicationTests {

    @Test
    fun symjaAndIndriyaLoad() {
        val expr = org.matheclipse.core.eval.ExprEvaluator().eval("N(Pi, 50)")
        val unit = tech.units.indriya.unit.Units.KILOGRAM.multiply(
            tech.units.indriya.unit.Units.METRE_PER_SQUARE_SECOND,
        )
        println("  [smoke] Symja: $expr")
        println("  [smoke] Indriya: $unit")
    }
}
