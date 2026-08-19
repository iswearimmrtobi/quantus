package app.tobiasschmidt.quantus.eval

import app.tobiasschmidt.quantus.cas.CasEngine
import app.tobiasschmidt.quantus.latex.LatexNumbers
import app.tobiasschmidt.quantus.quantity.PhysicalQuantity
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test
import org.matheclipse.core.expression.F
import tech.units.indriya.unit.Units
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

/** Phase 3: the internal eval facade — sessions, F5 variable definitions, F6 step generation. */
class EvalFacadeTest {

    private val facade = EvalFacade()

    @Test
    @DisplayName("milestone: a physics problem with variables produces a three-step LaTeX calculation")
    fun milestone() {
        val session = SessionId.random()
        facade.evaluate("m_1 = 2\\,\\mathrm{kg}", session)
        facade.evaluate("a_1 = 9{,}81\\,\\frac{m}{s^2}", session)

        val result = facade.evaluate("F = m_1 \\cdot a_1", session)

        assertNull(result.error)
        assertEquals(
            listOf(
                Step(StepKind.SYMBOLIC, "F = m_{1} \\cdot a_{1}"),
                Step(StepKind.SUBSTITUTED, "F = 2\\,\\mathrm{kg} \\cdot 9{,}81\\,\\frac{\\mathrm{m}}{\\mathrm{s}^{2}}"),
                Step(StepKind.RESULT, "F = 19{,}62\\,\\mathrm{N}"),
            ),
            result.steps,
        )
        assertEquals("19{,}62", result.value)
        assertEquals("N", result.unit)
    }

    @Test
    @DisplayName("a formula without variables has only a symbolic and a result step")
    fun noSubstitutedStepWithoutVariables() {
        val result = facade.evaluate("F = 2kg \\cdot 10\\frac{m}{s^2}", SessionId.random())
        assertEquals(listOf(StepKind.SYMBOLIC, StepKind.RESULT), result.steps.map { it.kind })
        assertEquals("F = 20\\,\\mathrm{N}", result.steps.last().latex)
    }

    @Test
    @DisplayName("a label without a subscript is just a name, not a variable definition")
    fun unindexedLabelIsNotStored() {
        val session = SessionId.random()
        facade.evaluate("F = 2kg \\cdot 10\\frac{m}{s^2}", session)
        assertTrue(facade.getVariables(session).isEmpty())
    }

    @Test
    @DisplayName("evaluating a labeled formula a second time updates the stored variable")
    fun redefiningUpdatesTheVariable() {
        val session = SessionId.random()
        facade.evaluate("r_1 = 0{,}5\\,\\mathrm{m}", session)
        facade.evaluate("r_1 = 2\\,\\mathrm{m}", session)

        val variables = facade.getVariables(session)
        assertEquals(1, variables.size)
        assertEquals(VariableKey("r", "1"), variables.single().key)
        assertEquals("2\\,\\mathrm{m}", facade.evaluate("r_1", session).steps.last().latex)
    }

    @Test
    @DisplayName("defineVariable stores a value directly, without going through LaTeX")
    fun defineVariableDirectly() {
        val session = SessionId.random()
        val quantity = PhysicalQuantity.of(5, Units.METRE)
        facade.defineVariable(session, "r", "1", quantity)

        assertEquals(listOf(VariableDefinition(VariableKey("r", "1"), quantity)), facade.getVariables(session))
        assertEquals("5\\,\\mathrm{m}", facade.evaluate("r_1", session).steps.last().latex)
    }

    @Test
    @DisplayName("an undefined variable comes back as an error, not an exception")
    fun undefinedVariableIsAnError() {
        val result = facade.evaluate("v_1 \\cdot 2s", SessionId.random())
        assertEquals("", result.value)
        assertEquals("", result.unit)
        assertNotNull(result.error)
        assertTrue(result.error!!.contains("v_1"), result.error!!)
        assertTrue(result.steps.isEmpty())
    }

    @Test
    @DisplayName("sessions are isolated: a variable defined in one session is unknown in another")
    fun sessionsAreIsolated() {
        val a = SessionId.random()
        val b = SessionId.random()
        facade.evaluate("r_1 = 0{,}5\\,\\mathrm{m}", a)

        assertTrue(facade.getVariables(b).isEmpty())
        assertNotNull(facade.evaluate("r_1", b).error)
    }

    @Test
    @DisplayName("a parse error also comes back as an error result, not a thrown exception")
    fun parseErrorIsAnError() {
        val result = facade.evaluate("F = )(", SessionId.random())
        assertNotNull(result.error)
    }

    @Test
    @DisplayName("Phase 4 milestone: (3+4i)(1-2i) = 11 - 2i, cartesian by default")
    fun complexMultiplicationCartesian() {
        val result = facade.evaluate("(3 + 4i)(1 - 2i)", SessionId.random())

        assertNull(result.error)
        assertEquals("11 - 2\\,i", result.value)
        assertEquals(Step(StepKind.RESULT, "11 - 2\\,i"), result.steps.last())
    }

    @Test
    @DisplayName("F4: the same complex result, rendered in polar form on request")
    fun complexMultiplicationPolar() {
        val result = facade.evaluate("(3 + 4i)(1 - 2i)", SessionId.random(), complexFormat = ComplexFormat.POLAR)

        assertNull(result.error)
        // |11 - 2i| = sqrt(125), arg = atan2(-2, 11) — checked against a decimal engine directly.
        val engine = CasEngine(50)
        val (r, phi) = engine.polarParts(F.Plus(F.ZZ(11), F.Times(F.ZZ(-2), F.CI)))
        assertEquals("${LatexNumbers.format(r)} \\cdot e^{i\\,${LatexNumbers.format(phi)}}", result.value)
    }

    @Test
    @DisplayName("a purely imaginary result renders without a spurious real part")
    fun pureImaginaryResult() {
        val result = facade.evaluate("2i \\cdot 3i", SessionId.random())
        assertEquals("-6", result.value)
    }

    @Test
    @DisplayName("Phase 4 milestone: d/dx of x^2 sin(x)")
    fun derivativeMilestone() {
        // Cross-checked against SymjaSpikeTest.calculus() and SymjaTexExportSpikeTest.calculusForms().
        val result = facade.derive("x^2 \\sin(x)", "x")

        assertNull(result.error)
        assertEquals(
            Step(StepKind.SYMBOLIC, "\\frac{d}{dx}\\left[x^{2} \\cdot \\sin\\left(x\\right)\\right]"),
            result.steps.first(),
        )
        assertEquals("{x}^{2} \\cdot \\cos (x)+2 \\cdot x \\cdot \\sin (x)", result.value)
        assertEquals(result.value, result.steps.last().latex)
        assertEquals("1", result.unit)
    }

    @Test
    @DisplayName("F8: higher-order derivatives")
    fun higherOrderDerivative() {
        val result = facade.derive("x^4", "x", order = 2)
        assertNull(result.error)
        assertEquals("12 \\cdot {x}^{2}", result.value)
        assertEquals(Step(StepKind.SYMBOLIC, "\\frac{d^{2}}{dx^{2}}\\left[x^{4}\\right]"), result.steps.first())
    }

    @Test
    @DisplayName("Phase 4: indefinite integral of x sin(x), with a + C")
    fun integralMilestone() {
        // Cross-checked against SymjaTexExportSpikeTest.calculusForms().
        val result = facade.integrate("x \\sin(x)", "x")

        assertNull(result.error)
        assertEquals("- x \\cdot \\cos (x) + \\sin (x)", result.value)
        assertEquals(Step(StepKind.RESULT, "- x \\cdot \\cos (x) + \\sin (x) + C"), result.steps.last())
    }

    @Test
    @DisplayName("F8: analysis expressions stay unitless for now")
    fun analysisIsUnitless() {
        // Explicit \mathrm{} markup still resolves to a real unit even in analysis mode.
        val result = facade.derive("x \\cdot \\mathrm{kg}", "x")
        assertNotNull(result.error)
        assertTrue(result.error!!.contains("unitless"), result.error!!)
    }
}
