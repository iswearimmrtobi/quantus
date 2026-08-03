package app.tobiasschmidt.quantus.eval

import app.tobiasschmidt.quantus.quantity.PhysicalQuantity
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test
import kotlin.test.assertEquals

/** F6 — the three states of a calculation: symbolic, substituted, result. */
class CalculationStepsTest {

    private val calculator = LatexCalculator()

    /** Evaluates a definition and keeps its value, the way a session does. */
    private fun define(latex: String): Pair<String, PhysicalQuantity> {
        val calculation = calculator.evaluate(latex)
        return calculation.formula.label!!.key!! to calculation.quantity
    }

    @Test
    @DisplayName("with variables the calculation shows all three steps")
    fun threeSteps() {
        val variables = mapOf(define("m_1 = 2kg"), define("a_1 = 9{,}81\\frac{m}{s^2}"))

        val calculation = calculator.evaluate("F = m_1 \\cdot a_1", variables)

        assertEquals(
            listOf(
                Step(StepKind.SYMBOLIC, "F = m_{1} \\cdot a_{1}"),
                Step(
                    StepKind.SUBSTITUTED,
                    "F = 2\\,\\mathrm{kg} \\cdot 9{,}81\\,\\frac{\\mathrm{m}}{\\mathrm{s}^{2}}",
                ),
                Step(StepKind.RESULT, "F = 19{,}62\\,\\mathrm{N}"),
            ),
            calculation.steps,
        )
    }

    @Test
    @DisplayName("without variables there is nothing to substitute — two steps")
    fun twoSteps() {
        val calculation = calculator.evaluate("F = 2kg \\cdot 10\\frac{m}{s^2}")

        assertEquals(listOf(StepKind.SYMBOLIC, StepKind.RESULT), calculation.steps.map { it.kind })
        assertEquals("F = 2\\,\\mathrm{kg} \\cdot 10\\,\\frac{\\mathrm{m}}{\\mathrm{s}^{2}}", calculation.steps[0].latex)
        assertEquals("F = 20\\,\\mathrm{N}", calculation.steps[1].latex)
    }

    @Test
    @DisplayName("substituted values keep their own unit and get brackets where needed")
    fun substitutionBrackets() {
        val variables = mapOf(define("m_1 = 2kg"), define("v_1 = \\frac{200km}{2{,}5h}"))

        val calculation = calculator.evaluate("E = \\frac{1}{2} \\cdot m_1 \\cdot v_1^2", variables)

        assertEquals(
            "E = \\frac{1}{2} \\cdot 2\\,\\mathrm{kg} \\cdot (80\\,\\frac{\\mathrm{km}}{\\mathrm{h}})^{2}",
            calculation.steps[1].latex,
        )
        assertEquals("493{,}82716049382716049382716049382716049382716049383\\,\\mathrm{J}", calculation.result)
    }

    @Test
    @DisplayName("an expression without a label carries no label into its steps")
    fun withoutLabel() {
        val calculation = calculator.evaluate("2kg + 500g")

        assertEquals("2\\,\\mathrm{kg} + 500\\,\\mathrm{g}", calculation.steps[0].latex)
        assertEquals("2{,}5\\,\\mathrm{kg}", calculation.steps[1].latex)
    }

    @Test
    @DisplayName("a definition that is already its own result shows one step, not two identical ones")
    fun singleStep() {
        val calculation = calculator.evaluate("m_1 = 2kg")

        assertEquals(listOf(Step(StepKind.RESULT, "m_{1} = 2\\,\\mathrm{kg}")), calculation.steps)
    }

    @Test
    @DisplayName("the whole calculation copies as one align* block (F7)")
    fun alignBlock() {
        val calculation = calculator.evaluate("F = 2kg \\cdot 10\\frac{m}{s^2}")

        assertEquals(
            """
            \begin{align*}
              F = 2\,\mathrm{kg} \cdot 10\,\frac{\mathrm{m}}{\mathrm{s}^{2}} \\
              F = 20\,\mathrm{N}
            \end{align*}
            """.trimIndent(),
            calculation.alignBlock,
        )
    }
}
