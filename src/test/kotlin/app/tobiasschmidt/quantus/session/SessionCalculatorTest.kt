package app.tobiasschmidt.quantus.session

import app.tobiasschmidt.quantus.QuantusException
import app.tobiasschmidt.quantus.QuantusProperties
import app.tobiasschmidt.quantus.eval.UnresolvedVariableException
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

/** F5 — variables live in the session and resolve in later inputs. */
class SessionCalculatorTest {

    private val calculator = SessionCalculator(QuantusProperties())
    private val session = Session("test")

    @Test
    @DisplayName("a definition is stored and resolves in the next calculation")
    fun defineAndUse() {
        calculator.evaluate(session, "m_1 = 2kg")
        calculator.evaluate(session, "a_1 = 9{,}81\\frac{m}{s^2}")

        val calculation = calculator.evaluate(session, "F = m_1 \\cdot a_1")

        assertEquals("F = 19{,}62\\,\\mathrm{N}", calculation.latex)
        assertEquals(listOf("m_1", "a_1"), session.variables().map { it.key })
        assertEquals("2\\,\\mathrm{kg}", session.variables().first().latex)
        assertEquals("m_{1}", session.variables().first().symbol)
    }

    @Test
    @DisplayName("a definition may build on variables already defined")
    fun chainedDefinition() {
        calculator.evaluate(session, "s_1 = 200km")
        calculator.evaluate(session, "t_1 = 2{,}5h")

        val calculation = calculator.evaluate(session, "v_1 = \\frac{s_1}{t_1}")

        assertEquals("v_1", calculation.formula.label?.key)
        // The units the user chose stay as they are — 200 km over 2.5 h is 80 km/h, not 22.2 m/s.
        val velocity = session.variables().last()
        assertEquals("km/h", velocity.unit)
        assertEquals("80\\,\\frac{\\mathrm{km}}{\\mathrm{h}}", velocity.latex)
    }

    @Test
    @DisplayName("redefining a variable replaces the old value in place")
    fun redefine() {
        calculator.evaluate(session, "r_1 = 0{,}5m")
        calculator.evaluate(session, "r_1 = 2m")

        assertEquals(1, session.variables().size)
        assertEquals("2\\,\\mathrm{m}", session.variables().single().latex)
    }

    @Test
    @DisplayName("a label without a subscript names the line but defines no variable (F2)")
    fun labelWithoutSubscript() {
        val calculation = calculator.evaluate(session, "F = 2kg \\cdot 10\\frac{m}{s^2}")

        assertEquals("F = 20\\,\\mathrm{N}", calculation.latex)
        assertTrue(session.variables().isEmpty())
        // Bare `F` keeps meaning farad afterwards — that is what the subscript rule buys.
        assertEquals("F", calculator.evaluate(session, "1F").unit)
    }

    @Test
    @DisplayName("defining via the variables endpoint rejects anything that is not a definition")
    fun defineRejectsExpressions() {
        val error = assertFailsWith<QuantusException> { calculator.define(session, "2kg \\cdot 3") }
        assertTrue(error.message!!.contains("does not define a variable"), error.message!!)

        val withoutSubscript = assertFailsWith<QuantusException> { calculator.define(session, "F = 2N") }
        assertTrue(withoutSubscript.message!!.contains("subscript"), withoutSubscript.message!!)
    }

    @Test
    @DisplayName("forgetting a variable makes it unresolvable again")
    fun forget() {
        calculator.evaluate(session, "r_1 = 0{,}5m")
        assertTrue(session.remove("r_1"))

        assertFailsWith<UnresolvedVariableException> { calculator.evaluate(session, "2 \\cdot r_1") }
    }

    @Test
    @DisplayName("the requested precision reaches the result, an impossible one is rejected")
    fun precision() {
        assertEquals("0{,}333333\\,\\mathrm{m}", calculator.evaluate(session, "\\frac{1m}{3}", precision = 6).result)

        val error = assertFailsWith<QuantusException> { calculator.evaluate(session, "1m", precision = 0) }
        assertTrue(error.message!!.contains("precision"), error.message!!)
    }
}
