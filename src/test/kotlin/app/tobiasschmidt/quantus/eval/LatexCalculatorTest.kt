package app.tobiasschmidt.quantus.eval

import app.tobiasschmidt.quantus.quantity.DimensionMismatchException
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test
import java.math.BigDecimal
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

/** Phase 2: LaTeX in, LaTeX out — the whole chain from input string to rendered result. */
class LatexCalculatorTest {

    private val calculator = LatexCalculator()

    private fun evaluate(latex: String) = calculator.evaluate(latex)

    @Test
    @DisplayName("milestone: F = 2kg \\cdot 10\\frac{m}{s^2} becomes F = 20\\,\\mathrm{N}")
    fun milestone() {
        val calculation = evaluate("F = 2kg \\cdot 10\\frac{m}{s^2}")

        assertEquals("F = 20\\,\\mathrm{N}", calculation.latex)
        assertEquals("20\\,\\mathrm{N}", calculation.result)
        assertEquals("F = 2\\,\\mathrm{kg} \\cdot 10\\,\\frac{\\mathrm{m}}{\\mathrm{s}^{2}}", calculation.input)
        assertEquals(0, BigDecimal("20").compareTo(calculation.value))
        assertEquals("N", calculation.unit)
        assertEquals("force", calculation.quantityName)
    }

    @Test
    @DisplayName("the spec's reference example: 1 kg × 1 m/s² = 1 N")
    fun referenceExample() {
        assertEquals("1\\,\\mathrm{N}", evaluate("1\\,\\mathrm{kg} \\times 1\\,\\mathrm{m/s^2}").latex)
    }

    @Test
    @DisplayName("textbook formulas end to end")
    fun formulas() {
        assertEquals("19{,}62\\,\\mathrm{N}", evaluate("2kg \\cdot 9{,}81\\frac{m}{s^2}").latex)
        assertEquals("E = 9\\,\\mathrm{J}", evaluate("E = \\frac{1}{2} \\cdot 2kg \\cdot (3\\frac{m}{s})^2").latex)
        assertEquals("U = 10\\,\\mathrm{V}", evaluate("U = 5\\Omega \\cdot 2A").latex)
        assertEquals("P = 50\\,\\mathrm{Pa}", evaluate("P = \\frac{100N}{2m^2}").latex)
        assertEquals("2{,}5\\,\\mathrm{kg}", evaluate("2kg + 500g").latex)
    }

    @Test
    @DisplayName("a result without a named unit keeps its compound unit and gets a quantity name")
    fun compoundResult() {
        val calculation = evaluate("v_1 = \\frac{100m}{8s}")
        assertEquals("12{,}5\\,\\frac{\\mathrm{m}}{\\mathrm{s}}", calculation.result)
        assertEquals("velocity", calculation.quantityName)
    }

    @Test
    @DisplayName("precision reaches the output: 50 digits of 1/3")
    fun precision() {
        val calculation = LatexCalculator(precision = 50).evaluate("\\frac{1m}{3}")
        assertTrue(
            calculation.result.startsWith("0{,}333333333333333333333333333333333333333333333333"),
            calculation.result,
        )
        assertEquals(50, calculation.value.precision())

        val short = LatexCalculator(precision = 6).evaluate("\\sqrt{2}m")
        assertEquals("1{,}41421\\,\\mathrm{m}", short.result)
    }

    @Test
    @DisplayName("very large results switch to scientific notation")
    fun scientificNotation() {
        val calculation = evaluate("1kg \\cdot (299792458\\frac{m}{s})^2")
        assertEquals("8{,}9875517873681764 \\cdot 10^{16}\\,\\mathrm{J}", calculation.result)
    }

    @Test
    @DisplayName("dimension errors survive the trip through the parser")
    fun dimensionErrors() {
        val error = assertFailsWith<DimensionMismatchException> { evaluate("1m + 1s") }
        assertEquals("Incompatible units: m (length) and s (time) cannot be added.", error.message)
    }

    @Test
    @DisplayName("degrees celsius survive the round trip, mixing with kelvin does not")
    fun temperatures() {
        val calculation = evaluate("20°C + 5°C")
        assertEquals("20\\,^\\circ\\mathrm{C} + 5\\,^\\circ\\mathrm{C}", calculation.input)
        assertEquals("25\\,^\\circ\\mathrm{C}", calculation.result)

        val error = assertFailsWith<DimensionMismatchException> { evaluate("20°C + 5K") }
        assertTrue(error.message!!.contains("zero point of its own"), error.message!!)
    }

    @Test
    @DisplayName("a variable without a definition says so — phase 3 will resolve them")
    fun undefinedVariable() {
        val error = assertFailsWith<UnresolvedVariableException> { evaluate("v_1 \\cdot 2s") }
        assertTrue(error.message!!.contains("v_1"), error.message!!)
    }
}
