package app.tobiasschmidt.quantus.latex

import java.math.BigDecimal

/**
 * Complex numbers in LaTeX, either cartesian (`a+bi`) or polar (`r \cdot e^{i\varphi}`) — F4.
 *
 * Reuses [LatexNumbers] for every individual number so both forms stay consistent with the
 * rest of the app: German decimal comma, scientific notation past the same thresholds.
 */
object LatexComplex {

    /** `a + b\,i`, collapsing a zero real or imaginary part instead of printing `+ 0\,i`. */
    fun cartesian(re: BigDecimal, im: BigDecimal): String {
        val real = if (re.signum() == 0 && im.signum() != 0) null else LatexNumbers.format(re)
        val imaginary = imaginaryTerm(im)
        return when {
            real == null && imaginary == null -> "0"
            real == null -> if (im.signum() < 0) "-$imaginary" else imaginary!!
            imaginary == null -> real
            im.signum() < 0 -> "$real - $imaginary"
            else -> "$real + $imaginary"
        }
    }

    /** `r \cdot e^{i\,\varphi}`, φ in radians. */
    fun polar(r: BigDecimal, phi: BigDecimal): String = "${LatexNumbers.format(r)} \\cdot e^{i\\,${LatexNumbers.format(phi)}}"

    /** The unsigned `b\,i` term, or `null` if `b` is zero. A magnitude of exactly 1 renders as bare `i`. */
    private fun imaginaryTerm(im: BigDecimal): String? {
        val magnitude = im.abs()
        if (magnitude.signum() == 0) return null
        if (magnitude.stripTrailingZeros().compareTo(BigDecimal.ONE) == 0) return "i"
        return "${LatexNumbers.format(magnitude)}\\,i"
    }
}
