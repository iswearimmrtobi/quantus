package app.tobiasschmidt.quantus.session

import app.tobiasschmidt.quantus.QuantusException
import app.tobiasschmidt.quantus.QuantusProperties
import app.tobiasschmidt.quantus.cas.CasEngine
import app.tobiasschmidt.quantus.eval.Calculation
import app.tobiasschmidt.quantus.eval.LatexCalculator
import app.tobiasschmidt.quantus.latex.LatexSerializer
import org.springframework.stereotype.Service

/**
 * Evaluation in the context of a session: variables of the session go in, and an input that
 * is itself a definition (`r_1 = 0{,}5m`, a label **with** a subscript per F5) comes back out
 * as a new definition.
 *
 * Each call builds its own [LatexCalculator] and with it a fresh [CasEngine] — the Symja
 * `EvalEngine` is thread-local and must not be shared between requests (phase 0, spike 1).
 */
@Service
class SessionCalculator(private val properties: QuantusProperties) {

    fun evaluate(session: Session, latex: String, precision: Int? = null): Calculation {
        val calculation = LatexCalculator(precision(precision)).evaluate(latex, session.quantities())
        val label = calculation.formula.label
        val key = label?.key
        if (key != null) {
            session.define(
                Variable(
                    key = key,
                    symbol = LatexSerializer.label(label),
                    latex = calculation.result,
                    value = calculation.value,
                    unit = calculation.unit,
                    quantity = calculation.quantity,
                ),
            )
        }
        return calculation
    }

    /**
     * Defines a variable from a complete definition (`r_1 = 0{,}5m`). Rejects inputs that
     * define nothing — an expression without a label, or a label without a subscript, which
     * F5 does not accept as a variable name.
     */
    fun define(session: Session, latex: String, precision: Int? = null): Variable {
        val calculation = evaluate(session, latex, precision)
        val key = calculation.formula.label?.key ?: throw QuantusException(
            "'$latex' does not define a variable. Write it as name and value, e.g. r_1 = 0{,}5m — " +
                "and remember that a variable name always carries a subscript.",
        )
        return session.variables().first { it.key == key }
    }

    private fun precision(requested: Int?): Int {
        val precision = requested ?: properties.precision
        if (precision !in 1..CasEngine.MAX_PRECISION) {
            throw QuantusException("The precision must be between 1 and ${CasEngine.MAX_PRECISION} digits, but was $precision.")
        }
        return precision
    }
}
