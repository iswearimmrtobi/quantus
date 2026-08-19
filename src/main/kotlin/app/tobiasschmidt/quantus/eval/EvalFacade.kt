package app.tobiasschmidt.quantus.eval

import app.tobiasschmidt.quantus.QuantusException
import app.tobiasschmidt.quantus.cas.CasEngine
import app.tobiasschmidt.quantus.cas.SymjaLatex
import app.tobiasschmidt.quantus.latex.Formula
import app.tobiasschmidt.quantus.latex.Label
import app.tobiasschmidt.quantus.latex.LatexComplex
import app.tobiasschmidt.quantus.latex.LatexNumbers
import app.tobiasschmidt.quantus.latex.LatexParser
import app.tobiasschmidt.quantus.latex.LatexSerializer
import app.tobiasschmidt.quantus.latex.Node
import app.tobiasschmidt.quantus.quantity.PhysicalQuantity
import app.tobiasschmidt.quantus.units.UnitRegistry
import org.matheclipse.core.expression.F
import org.matheclipse.core.interfaces.IExpr
import tech.units.indriya.AbstractUnit

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

    /**
     * F8 — a symbolic derivative, `order` times with respect to `wrt`. `expr` is a plain
     * analysis expression (`x^2\sin(x)`), not a labeled formula: bare identifiers need no
     * subscript here (see [LatexParser]'s `analysisMode`), and the result carries no unit —
     * analysis expressions are unitless for now (spec section 6).
     */
    fun derive(expr: String, wrt: String, order: Int = 1, precision: Int = CasEngine.DEFAULT_PRECISION): EvaluationResult {
        require(order > 0) { "order must be positive, was $order" }
        return try {
            val engine = CasEngine(precision)
            val integrand = analysisExpression(expr, engine)
            val variable = F.symbol(wrt)
            val derivative = engine.eval(F.D(integrand.value, if (order == 1) variable else F.List(variable, F.ZZ(order.toLong()))))
            val operator = if (order == 1) "\\frac{d}{d$wrt}" else "\\frac{d^{$order}}{d${wrt}^{$order}}"
            analysisResult(operator, integrand.node, derivative)
        } catch (e: QuantusException) {
            EvaluationResult(steps = emptyList(), value = "", unit = "", error = e.message)
        }
    }

    /** F8 — an indefinite integral with respect to `wrt`; same unitless scope as [derive]. */
    fun integrate(expr: String, wrt: String, precision: Int = CasEngine.DEFAULT_PRECISION): EvaluationResult {
        return try {
            val engine = CasEngine(precision)
            val integrand = analysisExpression(expr, engine)
            val antiderivative = engine.eval(F.Integrate(integrand.value, F.symbol(wrt)))
            val symbolicStep = "\\int ${LatexSerializer.expression(integrand.node)}\\,d$wrt"
            val resultStep = "${SymjaLatex.format(antiderivative)} + C"
            EvaluationResult(
                steps = listOf(Step(StepKind.SYMBOLIC, symbolicStep), Step(StepKind.RESULT, resultStep)),
                value = SymjaLatex.format(antiderivative),
                unit = UnitRegistry.format(AbstractUnit.ONE),
                error = null,
            )
        } catch (e: QuantusException) {
            EvaluationResult(steps = emptyList(), value = "", unit = "", error = e.message)
        }
    }

    /** Parses and evaluates an analysis expression (F8), rejecting one that carries a unit. */
    private fun analysisExpression(expr: String, engine: CasEngine): AnalysisExpression {
        val formula = LatexParser(expr, analysisMode = true).parse()
        val quantity = ExpressionEvaluator().evaluate(formula.expression)
        if (!quantity.isDimensionless) {
            throw QuantusException(
                "Analysis expressions are unitless for now, but '$expr' carries the unit " +
                    UnitRegistry.describe(quantity.unit) + ".",
            )
        }
        return AnalysisExpression(formula.expression, quantity.value)
    }

    private fun analysisResult(operatorLatex: String, integrand: Node, result: IExpr): EvaluationResult {
        val resultLatex = SymjaLatex.format(result)
        val symbolicStep = "$operatorLatex\\left[${LatexSerializer.expression(integrand)}\\right]"
        return EvaluationResult(
            steps = listOf(Step(StepKind.SYMBOLIC, symbolicStep), Step(StepKind.RESULT, resultLatex)),
            value = resultLatex,
            unit = UnitRegistry.format(AbstractUnit.ONE),
            error = null,
        )
    }

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

/** F8 — a parsed analysis expression: its AST (for the symbolic step) and its Symja value. */
private data class AnalysisExpression(val node: Node, val value: IExpr)

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
