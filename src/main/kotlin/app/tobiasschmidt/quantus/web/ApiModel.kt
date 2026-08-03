package app.tobiasschmidt.quantus.web

import app.tobiasschmidt.quantus.eval.Calculation
import app.tobiasschmidt.quantus.session.Variable

/** `POST /api/evaluate` and `POST /api/variables` — the input as the user typed it. */
data class CalculationRequest(
    val latex: String,
    val sessionId: String? = null,
    /** Significant digits for this one call; `null` uses the configured default. */
    val precision: Int? = null,
)

/**
 * One evaluated input, ready for display: the steps of F6, the result in LaTeX and as a
 * plain number, plus what the input did to the session.
 */
data class CalculationResponse(
    /** The input re-rendered canonically (`/` as `\frac{}{}`). */
    val input: String,
    /** The finished line: `F = 20\,\mathrm{N}`. */
    val latex: String,
    /** Just the result, without the label: `20\,\mathrm{N}`. */
    val result: String,
    val steps: List<StepResponse>,
    /** The whole calculation as a copyable `align*` block (F7). */
    val alignBlock: String,
    /** The numeric value in full precision, unformatted. */
    val value: String,
    val unit: String,
    /** The informational name of the quantity (F11), e.g. "force" — `null` if unknown. */
    val quantityName: String?,
    /** The dimension as Indriya writes it, e.g. `[M]·[L]/[T]²`. */
    val dimension: String,
    /** Set when the input defined a variable (`r_1 = 0{,}5m`). */
    val variable: VariableResponse?,
) {

    companion object {
        fun of(calculation: Calculation, variable: Variable?) = CalculationResponse(
            input = calculation.input,
            latex = calculation.latex,
            result = calculation.result,
            steps = calculation.steps.map { StepResponse(it.kind.name.lowercase(), it.latex) },
            alignBlock = calculation.alignBlock,
            value = calculation.value.toPlainString(),
            unit = calculation.unit,
            quantityName = calculation.quantityName,
            dimension = calculation.quantity.dimension.toString(),
            variable = variable?.let { VariableResponse.of(it) },
        )
    }
}

/** [kind] is `symbolic`, `substituted` or `result`. */
data class StepResponse(val kind: String, val latex: String)

data class VariableResponse(
    /** `r_1` — the key the session uses. */
    val key: String,
    /** `r_{1}` — the name in LaTeX. */
    val symbol: String,
    /** `0{,}5\,\mathrm{m}` — value and unit in LaTeX. */
    val latex: String,
    val value: String,
    val unit: String,
) {

    companion object {
        fun of(variable: Variable) = VariableResponse(
            key = variable.key,
            symbol = variable.symbol,
            latex = variable.latex,
            value = variable.value.toPlainString(),
            unit = variable.unit,
        )
    }
}

/** Everything that went wrong is reported like this — [position] only for parse errors. */
data class ErrorResponse(val error: String, val position: Int? = null)
