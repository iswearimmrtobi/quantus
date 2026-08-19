package app.tobiasschmidt.quantus.cas

import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test
import org.matheclipse.core.expression.F
import java.math.BigDecimal
import java.math.MathContext
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

/** Phase 1: precision (F3) travels all the way into Symja. */
class CasEngineTest {

    @Test
    @DisplayName("decimal input stays exact: 9.81 becomes 981/100, never a double")
    fun decimalInputIsKeptRational() {
        assertEquals("981/100", exactExpr(BigDecimal("9.81")).toString())
        assertEquals("2", exactExpr(BigDecimal("2.00")).toString())
        assertEquals("-1/8", exactExpr(BigDecimal("-0.125")).toString())
        assertEquals("42", exactExpr(42).toString())
    }

    @Test
    @DisplayName("50 significant digits by default")
    fun defaultPrecision() {
        val engine = CasEngine()
        assertEquals(50, engine.precision)

        val sqrt2 = engine.decimal(F.Sqrt(F.C2))
        assertTrue(
            sqrt2.toPlainString().startsWith("1.4142135623730950488016887242096980785696718753769"),
            "expected 50 digits of sqrt(2), got $sqrt2",
        )
        assertEquals(50, sqrt2.precision())
    }

    @Test
    @DisplayName("the digit count is configurable")
    fun configurablePrecision() {
        assertEquals("3.14159265358979", CasEngine(15).decimal(F.Pi).toPlainString())
        assertEquals(200, CasEngine(200).decimal(F.Divide(F.C1, F.ZZ(3))).precision())
        assertFailsWith<IllegalArgumentException> { CasEngine(0) }
    }

    @Test
    @DisplayName("exact input plus late numerics: no double rounding sneaks in")
    fun exactArithmeticSurvivesToTheDecimalBoundary() {
        val engine = CasEngine()
        // 0.1 + 0.2 is the classic double trap; as 1/10 + 1/5 it is exactly 0.3.
        val sum = engine.eval(F.Plus(exactExpr(BigDecimal("0.1")), exactExpr(BigDecimal("0.2"))))
        assertEquals("3/10", sum.toString())
        assertEquals(0, BigDecimal("0.3").compareTo(engine.decimal(sum)))

        val third = engine.decimal(F.Divide(F.C1, F.ZZ(3)))
        assertEquals(0, BigDecimal.ONE.divide(BigDecimal(3), MathContext(50)).compareTo(third))
    }

    @Test
    @DisplayName("a complex value has no decimal representation")
    fun complexValueIsRejected() {
        val error = assertFailsWith<NotRealException> { CasEngine().decimal(F.Plus(F.C1, F.CI)) }
        assertTrue(error.message!!.contains("not a real number"), error.message!!)
    }

    @Test
    @DisplayName("F4: a complex value splits into real and imaginary decimal parts")
    fun complexParts() {
        val engine = CasEngine(10)
        val (re, im) = engine.complexParts(F.Plus(F.ZZ(11), F.Times(F.ZZ(-2), F.CI)))
        assertEquals(0, BigDecimal("11").compareTo(re))
        assertEquals(0, BigDecimal("-2").compareTo(im))
    }

    @Test
    @DisplayName("F4: a complex value splits into modulus and argument (polar form)")
    fun polarParts() {
        val engine = CasEngine(10)
        // 3 + 4i has modulus 5, exactly.
        val (r, phi) = engine.polarParts(F.Plus(F.ZZ(3), F.Times(F.ZZ(4), F.CI)))
        assertEquals(0, BigDecimal("5").compareTo(r))
        assertTrue(phi.signum() > 0, "3+4i lies in the first quadrant, expected a positive argument")
    }
}
