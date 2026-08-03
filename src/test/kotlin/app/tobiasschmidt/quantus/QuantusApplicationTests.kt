package app.tobiasschmidt.quantus

import org.junit.jupiter.api.Test
import org.springframework.boot.test.context.SpringBootTest

/** Meilenstein Phase 0: Spring Boot startet mit Symja und Indriya im selben Classpath. */
@SpringBootTest
class QuantusApplicationTests {

    @Test
    fun contextLoads() {
        // Smoke-Test: beide Libraries sind ladbar, keine Classpath-Kollision.
        val expr = org.matheclipse.core.eval.ExprEvaluator().eval("N(Pi, 50)")
        val unit = tech.units.indriya.unit.Units.KILOGRAM.multiply(
            tech.units.indriya.unit.Units.METRE_PER_SQUARE_SECOND,
        )
        println("  [smoke] Symja: $expr")
        println("  [smoke] Indriya: $unit")
    }
}
