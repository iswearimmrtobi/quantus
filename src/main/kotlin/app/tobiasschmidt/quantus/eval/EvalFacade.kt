package app.tobiasschmidt.quantus.eval

import app.tobiasschmidt.quantus.QuantusException
import app.tobiasschmidt.quantus.cas.CasEngine
import app.tobiasschmidt.quantus.latex.Formula
import app.tobiasschmidt.quantus.latex.Label
import app.tobiasschmidt.quantus.latex.LatexComplex
import app.tobiasschmidt.quantus.latex.LatexNumbers
import app.tobiasschmidt.quantus.latex.LatexParser
import app.tobiasschmidt.quantus.latex.LatexSerializer
import app.tobiasschmidt.quantus.quantity.PhysicalQuantity
import app.tobiasschmidt.quantus.units.UnitRegistry

/**
 * The internal facade the UI (phase 5) calls directly — spec section 5. No REST/JSON (N2a):
 * this is the whole interface between the JavaFX layer and the LaTeX/CAS/units core.
 *
 * Ties [SessionStore] to the latex -> AST -> [PhysicalQuantity] -> latex pipeline from phase 2
 * and adds the three-step presentation from F6. A formula labeled with a subscript
 * (`r_1 = 0{,}5\,\mathrm{m}`) defines a variable as a side effect of evaluating it — that is
 * how F5 examples read, and it lets a later step reuse an earlier result by name.
 */
class EvalFacade(private val sessions: SessionStore = SessionStore()) {

    fun evaluate(
        latex: String,
        sessionId: SessionId,
        precision: Int = CasEngine.DEFAULT_PRECISION,
        complexFormat: ComplexFormat = ComplexFormat.CARTESIAN,
    ): EvaluationResult {
        val session = sessions.session(sessionId)
        return try {
            evaluateOrThrow(latex, session, precision, complexFormat)
        } catch (e: QuantusException) {
            EvaluationResult(steps = emptyList(), value = "", unit = "", error = e.message)
        }
    }

    fun defineVariable(sessionId: SessionId, name: String, index: String, value: PhysicalQuantity) {
        sessions.session(sessionId).define(VariableKey(name, index), value)
    }

    fun getVariables(sessionId: SessionId): List<VariableDefinition> = sessions.session(sessionId).variables()

    private fun evaluateOrThrow(latex: String, session: Session, precision: Int, complexFormat: ComplexFormat): EvaluationResult {
        // The engine has to exist before any expression is built: it installs the
        // thread-local EvalEngine that PhysicalQuantity's arithmetic uses.
        val engine = CasEngine(precision)
        val formula = LatexParser(latex).parse()
        val quantity = ExpressionEvaluator(session::resolve).evaluate(formula.expression).simplify()
        val (numberLatex, quantityLatex) = render(quantity, engine, complexFormat)

        val steps = buildSteps(formula, session, engine, resultLatex(formula.label, quantityLatex))

        val label = formula.label
        if (label != null && label.index != null) {
            session.define(VariableKey(label.name, label.index), quantity)
        }

        return EvaluationResult(
            steps = steps,
            value = numberLatex,
            unit = UnitRegistry.format(quantity.unit),
            error = null,
        )
    }

    /** The bare number and the number-plus-unit LaTeX, real or complex (F4). */
    private fun render(quantity: PhysicalQuantity, engine: CasEngine, complexFormat: ComplexFormat): Pair<String, String> {
        if (!quantity.isComplex) {
            val value = quantity.decimal(engine)
            return LatexNumbers.format(value) to LatexSerializer.quantity(value, quantity.unit)
        }
        return when (complexFormat) {
            ComplexFormat.CARTESIAN -> {
                val (re, im) = quantity.complexCartesian(engine)
                LatexComplex.cartesian(re, im) to LatexSerializer.complexQuantity(re, im, quantity.unit)
            }

            ComplexFormat.POLAR -> {
                val (r, phi) = quantity.complexPolar(engine)
                LatexComplex.polar(r, phi) to LatexSerializer.polarQuantity(r, phi, quantity.unit)
            }
        }
    }

    private fun buildSteps(formula: Formula, session: Session, engine: CasEngine, resultBody: String): List<Step> = buildList {
        add(Step(StepKind.SYMBOLIC, LatexSerializer.formula(formula)))

        VariableSubstitution.substitute(formula.expression, session, engine)?.let { substituted ->
            add(Step(StepKind.SUBSTITUTED, LatexSerializer.formula(Formula(formula.label, substituted, formula.source))))
        }

        add(Step(StepKind.RESULT, resultBody))
    }

    private fun resultLatex(label: Label?, quantityLatex: String): String =
        label?.let { "${LatexSerializer.label(it)} = $quantityLatex" } ?: quantityLatex
}

/** F4 — how a complex result is written: cartesian `a+bi` or polar `r \cdot e^{iφ}`. */
enum class ComplexFormat { CARTESIAN, POLAR }

/** F6: a calculation's three-step LaTeX presentation, symbolic -> substituted -> result. */
enum class StepKind { SYMBOLIC, SUBSTITUTED, RESULT }

data class Step(val kind: StepKind, val latex: String)

/**
 * [error] is set instead of throwing so the UI can show a message without a try/catch of its
 * own; on error [steps], [value] and [unit] are empty. On success [value] and [unit] mirror
 * the last (RESULT) step for callers that only want the number.
 */
data class EvaluationResult(
    val steps: List<Step>,
    val value: String,
    val unit: String,
    val error: String?,
)
