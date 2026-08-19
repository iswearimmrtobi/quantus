package app.tobiasschmidt.quantus.eval

import app.tobiasschmidt.quantus.cas.CasEngine
import app.tobiasschmidt.quantus.latex.LatexParser
import app.tobiasschmidt.quantus.latex.LatexSerializer
import app.tobiasschmidt.quantus.quantity.PhysicalQuantity
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test
import tech.units.indriya.unit.Units
import java.math.BigDecimal
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNull

/** Phase 3, F6 step 2: rewriting variables in the AST to their session values. */
class VariableSubstitutionTest {

    private val engine = CasEngine()

    @Test
    @DisplayName("a formula without variables has nothing to substitute")
    fun noVariables() {
        val formula = LatexParser("2kg \\cdot 10\\frac{m}{s^2}").parse()
        assertNull(VariableSubstitution.substitute(formula.expression, Session(SessionId.random()), engine))
    }

    @Test
    @DisplayName("F = m \\cdot a becomes F = 2\\,kg \\cdot 9.81\\,m/s^2, unevaluated")
    fun substitutesDefinedVariables() {
        val session = Session(SessionId.random())
        session.define(VariableKey("m", "1"), PhysicalQuantity.of(2, Units.KILOGRAM))
        session.define(VariableKey("a", "1"), PhysicalQuantity.of(BigDecimal("9.81"), Units.METRE_PER_SQUARE_SECOND))

        val formula = LatexParser("F = m_1 \\cdot a_1").parse()
        val substituted = VariableSubstitution.substitute(formula.expression, session, engine)!!
        assertEquals(
            "2\\,\\mathrm{kg} \\cdot 9{,}81\\,\\frac{\\mathrm{m}}{\\mathrm{s}^{2}}",
            LatexSerializer.expression(substituted),
        )
    }

    @Test
    @DisplayName("a squared variable gets brackets around its substituted value")
    fun squaredVariableGetsBrackets() {
        val session = Session(SessionId.random())
        session.define(VariableKey("v", "1"), PhysicalQuantity.of(3, Units.METRE_PER_SECOND))

        val formula = LatexParser("v_1^2").parse()
        val substituted = VariableSubstitution.substitute(formula.expression, session, engine)!!
        assertEquals("(3\\,\\frac{\\mathrm{m}}{\\mathrm{s}})^{2}", LatexSerializer.expression(substituted))
    }

    @Test
    @DisplayName("an undefined variable fails the same way as plain evaluation")
    fun undefinedVariableFails() {
        val formula = LatexParser("v_1 \\cdot 2s").parse()
        val error = assertFailsWith<UnresolvedVariableException> {
            VariableSubstitution.substitute(formula.expression, Session(SessionId.random()), engine)
        }
        kotlin.test.assertTrue(error.message!!.contains("v_1"), error.message!!)
    }
}
