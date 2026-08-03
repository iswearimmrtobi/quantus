package app.tobiasschmidt.quantus.latex

import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNull
import kotlin.test.assertTrue

/**
 * Phase 2: parser and serialiser, tested as a round trip. What goes in as LaTeX comes back
 * as canonical LaTeX — that covers the tokeniser, the grammar, unit recognition, the
 * auto-fraction rewrite and the output markup in one go.
 */
class LatexParserTest {

    private fun roundTrip(input: String): String = LatexSerializer.formula(LatexParser(input).parse())

    private fun parseError(input: String) = assertFailsWith<LatexParseException> { LatexParser(input).parse() }

    @Test
    @DisplayName("the phase 2 milestone parses and renders")
    fun milestone() {
        assertEquals(
            "F = 2\\,\\mathrm{kg} \\cdot 10\\,\\frac{\\mathrm{m}}{\\mathrm{s}^{2}}",
            roundTrip("F = 2kg \\cdot 10\\frac{m}{s^2}"),
        )
    }

    @Test
    @DisplayName("units without markup: a number next to a unit is a product")
    fun bareUnits() {
        assertEquals("2\\,\\mathrm{kg}", roundTrip("2kg"))
        assertEquals("9\\,\\mathrm{mN}", roundTrip("9mN"))
        assertEquals("5\\,\\mathrm{\\mu m}", roundTrip("5\\mu m"))
        assertEquals("3\\,\\mathrm{kJ}", roundTrip("3kJ"))
        assertEquals("7\\,\\mathrm{\\Omega }", roundTrip("7\\Omega"))
    }

    @Test
    @DisplayName("explicit markup stays supported: \\mathrm, \\text, siunitx")
    fun explicitMarkup() {
        assertEquals("2\\,\\mathrm{kg}", roundTrip("2\\,\\mathrm{kg}"))
        assertEquals("2\\,\\mathrm{kg}", roundTrip("2\\text{kg}"))
        assertEquals("12\\,\\mathrm{cm}", roundTrip("\\qty{12}{cm}"))
        assertEquals("10\\,\\frac{\\mathrm{m}}{\\mathrm{s}^{2}}", roundTrip("10\\mathrm{m/s^2}"))
    }

    @Test
    @DisplayName("German decimal commas, in every spelling")
    fun decimalCommas() {
        assertEquals("9{,}81\\,\\frac{\\mathrm{m}}{\\mathrm{s}^{2}}", roundTrip("9{,}81\\frac{m}{s^2}"))
        assertEquals("0{,}5\\,\\mathrm{m}", roundTrip("0,5m"))
        assertEquals("0{,}5\\,\\mathrm{m}", roundTrip("0.5m"))
    }

    @Test
    @DisplayName("auto-fraction: every / becomes a \\frac, chains share one denominator")
    fun autoFractions() {
        assertEquals("\\frac{v_{1}}{t_{1}}", roundTrip("v_1/t_1"))
        assertEquals("\\frac{v_{1}}{t_{1} \\cdot x_{1}}", roundTrip("v_1/t_1/x_1"))
        assertEquals("\\frac{2 \\cdot 3}{4}", roundTrip("2 \\cdot 3/4"))
        // explicit brackets win
        assertEquals("\\frac{v_{1}}{\\frac{t_{1}}{x_{1}}}", roundTrip("v_1/(t_1/x_1)"))
        assertEquals("\\frac{v_{1} \\cdot t_{1}}{x_{1}}", roundTrip("(v_1 \\cdot t_1)/x_1"))
    }

    @Test
    @DisplayName("variables carry a subscript, labels may go without")
    fun variablesAndLabels() {
        val formula = LatexParser("E_{kin} = \\frac{1}{2} \\cdot m_1 \\cdot v_1^2").parse()
        assertEquals("E", formula.label?.name)
        assertEquals("kin", formula.label?.index)
        assertEquals(
            "E_{kin} = \\frac{1}{2} \\cdot m_{1} \\cdot v_{1}^{2}",
            LatexSerializer.formula(formula),
        )

        assertEquals("\\omega_{0} \\cdot t_{1}", roundTrip("\\omega_0 \\cdot t_1"))
        assertNull(LatexParser("2kg").parse().label)
    }

    @Test
    @DisplayName("brackets are re-derived from precedence, not copied")
    fun brackets() {
        assertEquals("(2 + 3) \\cdot 4", roundTrip("(2+3) \\cdot 4"))
        assertEquals("2 + 3 \\cdot 4", roundTrip("2+(3 \\cdot 4)"))
        assertEquals("2 - (3 - 4)", roundTrip("2-(3-4)"))
        assertEquals("(2 + 3)^{2}", roundTrip("(2+3)^2"))
        assertEquals("-5\\,\\mathrm{m}", roundTrip("-5m"))
    }

    @Test
    @DisplayName("roots, powers and constants")
    fun rootsPowersConstants() {
        assertEquals("\\sqrt{2}", roundTrip("\\sqrt{2}"))
        assertEquals("\\sqrt[3]{8}", roundTrip("\\sqrt[3]{8}"))
        assertEquals("2 \\cdot \\pi \\cdot r_{1}", roundTrip("2 \\cdot \\pi \\cdot r_1"))
        assertEquals("2^{3^{2}}", roundTrip("2^3^2"))
        assertEquals("x_{1}^{-1}", roundTrip("x_1^{-1}"))
    }

    @Test
    @DisplayName("parse errors carry a position and say what is wrong")
    fun parseErrors() {
        // mgh: neither a unit nor a constant, and without a subscript not a variable either
        val unknown = parseError("2 \\cdot mgh")
        assertEquals(8, unknown.position)
        assertTrue(unknown.reason.contains("neither a unit nor a constant"), unknown.reason)
        assertTrue(unknown.message!!.contains("position 9"), unknown.message!!)

        val incomplete = parseError("2 +")
        assertTrue(incomplete.reason.contains("Expected a number"), incomplete.reason)

        val missingArgument = parseError("\\frac{1}")
        assertTrue(missingArgument.reason.contains("braces"), missingArgument.reason)

        val unknownUnit = parseError("5\\mathrm{xyz}")
        assertEquals(9, unknownUnit.position)
        assertTrue(unknownUnit.reason.contains("not a known unit"), unknownUnit.reason)

        val unknownCommand = parseError("2 \\foo 3")
        assertTrue(unknownCommand.reason.contains("Unsupported command"), unknownCommand.reason)

        assertTrue(parseError("").reason.contains("empty"))
        assertTrue(parseError("2 = 3 = 4").reason.contains("Only one '='"))
    }

    @Test
    @DisplayName("implicit unit products are refused, longest match wins")
    fun implicitUnitProducts() {
        // `m s` is millisecond by longest match — not metre·second.
        assertEquals("2\\,\\mathrm{ms}", roundTrip("2m s"))
        // A product of units needs an explicit \cdot.
        assertEquals("2\\,\\mathrm{m} \\cdot \\mathrm{s}", roundTrip("2m \\cdot s"))
        // And a run that resolves to nothing is an error rather than a guess.
        assertTrue(parseError("2kg m").reason.contains("explicit \\cdot"))
    }
}
