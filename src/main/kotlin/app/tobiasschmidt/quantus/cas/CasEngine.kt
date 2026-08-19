package app.tobiasschmidt.quantus.cas

import app.tobiasschmidt.quantus.QuantusException
import org.matheclipse.core.eval.ExprEvaluator
import org.matheclipse.core.expression.F
import org.matheclipse.core.interfaces.IExpr
import java.math.BigDecimal
import java.math.MathContext

/**
 * The single point of contact with Symja where precision (F3) matters.
 *
 * Two findings from phase 0 are hard-wired here:
 * 1. `setNumericPrecision()` alone is not enough — the digit count has to be passed as the
 *    second argument to `N(...)`. Every numeric evaluation therefore goes through [numeric].
 * 2. `EvalEngine` is thread-local. A `CasEngine` belongs to exactly one thread resp. one
 *    request; it must **not** be shared across requests as a singleton bean.
 *
 * Constructing the [ExprEvaluator] installs the thread-local `EvalEngine` — the symbolic
 * arithmetic in `PhysicalQuantity` (via `F.eval`) uses that same engine.
 */
class CasEngine(val precision: Int = DEFAULT_PRECISION) {

    init {
        require(precision in 1..MAX_PRECISION) { "precision must be between 1 and $MAX_PRECISION, was: $precision" }
    }

    private val evaluator = ExprEvaluator()

    /** Symbolic evaluation without numerics: rationals stay rational. */
    fun eval(expr: IExpr): IExpr = evaluator.eval(expr)

    /** Numeric evaluation with the configured digit count: `N(expr, precision)`. */
    fun numeric(expr: IExpr): IExpr = evaluator.eval(F.N(expr, precision.toLong()))

    /**
     * The value as a decimal with [precision] significant digits, correctly rounded.
     *
     * Apfloat truncates the last digit instead of rounding it (5/18 comes out as
     * `0.2777…777` rather than `…778`), so the evaluation asks for a few guard digits and
     * the rounding happens here. The detour via `Apfloat.toString()` is lossless
     * (phase 0, spike 1).
     */
    fun decimal(expr: IExpr): BigDecimal {
        val evaluated = evaluator.eval(F.N(expr, (precision + GUARD_DIGITS).toLong()))
        val real = evaluated.evalReal()
            ?: throw NotRealException("'$evaluated' is not a real number and cannot be shown as a decimal.")
        return BigDecimal(real.apfloatValue().toString()).round(MathContext(precision))
    }

    /**
     * A complex value's real and imaginary part, each rounded to [precision] digits (F4).
     * `Re`/`Im` of a complex number are themselves real, so this reuses [decimal] verbatim.
     */
    fun complexParts(expr: IExpr): Pair<BigDecimal, BigDecimal> = decimal(F.Re(expr)) to decimal(F.Im(expr))

    /** The same value as modulus and argument (radians), for the polar form `r * e^{iφ}` (F4). */
    fun polarParts(expr: IExpr): Pair<BigDecimal, BigDecimal> = decimal(F.Abs(expr)) to decimal(F.Arg(expr))

    companion object {
        /** Default digit count per F3; overridable via `quantus.precision`. */
        const val DEFAULT_PRECISION = 50
        const val MAX_PRECISION = 10_000

        /** Extra digits computed so the last reported digit is rounded, not truncated. */
        private const val GUARD_DIGITS = 5
    }
}

/** An expression was expected to evaluate to a real number but did not (e.g. a complex result). */
class NotRealException(message: String) : QuantusException(message)
