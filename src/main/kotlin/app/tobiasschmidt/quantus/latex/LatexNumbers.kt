package app.tobiasschmidt.quantus.latex

import java.math.BigDecimal

/**
 * Numbers in LaTeX, written the way a physics class writes them (F6): `19{,}62`.
 *
 * The comma is typeset as `{,}` so LaTeX keeps it tight instead of adding the spacing it
 * would put after a list separator. Very large and very small magnitudes switch to
 * scientific notation with a proper `\cdot 10^{…}`.
 */
object LatexNumbers {

    private const val DECIMAL_SEPARATOR = "{,}"
    private val SCIENTIFIC_UPPER = BigDecimal("1E+15")
    private val SCIENTIFIC_LOWER = BigDecimal("1E-4")

    fun format(value: BigDecimal): String {
        val stripped = value.stripTrailingZeros()
        if (stripped.signum() == 0) return "0"

        val magnitude = stripped.abs()
        return if (magnitude >= SCIENTIFIC_UPPER || magnitude < SCIENTIFIC_LOWER) {
            scientific(stripped)
        } else {
            plain(stripped)
        }
    }

    private fun plain(value: BigDecimal): String = value.toPlainString().replace(".", DECIMAL_SEPARATOR)

    private fun scientific(value: BigDecimal): String {
        val exponent = value.precision() - value.scale() - 1
        val mantissa = value.movePointLeft(exponent).stripTrailingZeros()
        return "${plain(mantissa)} \\cdot 10^{$exponent}"
    }
}
