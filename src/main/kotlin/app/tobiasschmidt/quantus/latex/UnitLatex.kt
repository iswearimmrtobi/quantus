package app.tobiasschmidt.quantus.latex

import tech.units.indriya.AbstractUnit
import tech.units.indriya.format.SimpleUnitFormat
import tech.units.indriya.unit.ProductUnit
import javax.measure.Unit

/**
 * Units as LaTeX — upright via `\mathrm{}`, fractions as `\frac{}{}` (F2, F7).
 *
 * Compound units are rendered from Indriya's [ProductUnit] structure rather than from its
 * formatted string: the components come with their exponents attached, so `kg/(m·s²)` needs
 * no bracket parsing and `m²/s²` no superscript decoding.
 */
object UnitLatex {

    /** The unit of a result, e.g. `\mathrm{N}` or `\frac{\mathrm{kg}}{\mathrm{m}^{3}}`. */
    fun format(unit: Unit<*>): String {
        if (unit == AbstractUnit.ONE) return ""
        if (unit !is ProductUnit<*>) return symbol(SimpleUnitFormat.getInstance().format(unit))

        val numerator = mutableListOf<String>()
        val denominator = mutableListOf<String>()
        for (i in 0 until unit.unitCount) {
            val component = symbol(SimpleUnitFormat.getInstance().format(unit.getUnit(i)))
            val power = unit.getUnitPow(i)
            val root = unit.getUnitRoot(i)
            val target = if (power < 0) denominator else numerator
            target += withExponent(component, kotlin.math.abs(power), root)
        }

        val top = if (numerator.isEmpty()) "1" else numerator.joinToString(" \\cdot ")
        if (denominator.isEmpty()) return top
        return "\\frac{$top}{${denominator.joinToString(" \\cdot ")}}"
    }

    /**
     * A bare unit symbol as it was written in the input (`kg`, `µm`, `°C`), marked up as
     * `\mathrm{}`. Symbols Indriya prints with special characters keep their LaTeX spelling.
     */
    fun symbol(symbol: String): String {
        if (symbol.contains('/')) {
            // Symbols like `km/h` carry their own fraction.
            val numerator = symbol.substringBefore('/')
            val denominator = symbol.substringAfter('/').removeSurrounding("(", ")")
            return "\\frac{${symbol(numerator)}}{${symbol(denominator)}}"
        }
        if (symbol.contains('·')) {
            return symbol.split('·').joinToString(" \\cdot ") { symbol(it) }
        }
        DEGREE_SYMBOLS[symbol]?.let { return "^\\circ\\mathrm{$it}" }

        val exponent = trailingExponent(symbol)
        val base = symbol.dropLast(exponent?.first ?: 0)
        val markup = "\\mathrm{${escape(base)}}"
        return if (exponent == null) markup else "$markup^{${exponent.second}}"
    }

    private fun withExponent(component: String, power: Int, root: Int): String = when {
        power == 1 && root == 1 -> component
        root == 1 -> "$component^{$power}"
        else -> "$component^{$power/$root}"
    }

    /** Unicode superscripts as Indriya writes them (`m²`), as a (length, exponent) pair. */
    private fun trailingExponent(symbol: String): Pair<Int, String>? {
        val digits = symbol.takeLastWhile { it in SUPERSCRIPTS }
        if (digits.isEmpty()) return null
        val exponent = digits.map { SUPERSCRIPTS.getValue(it) }.joinToString("")
        return digits.length to exponent
    }

    private fun escape(symbol: String): String = symbol.map { ESCAPES[it] ?: it.toString() }.joinToString("")

    private val SUPERSCRIPTS = mapOf(
        '⁰' to "0", '¹' to "1", '²' to "2", '³' to "3", '⁴' to "4",
        '⁵' to "5", '⁶' to "6", '⁷' to "7", '⁸' to "8", '⁹' to "9", '⁻' to "-",
    )

    /** Indriya prints degree celsius as the single character `℃`; LaTeX wants `^\circ\mathrm{C}`. */
    private val DEGREE_SYMBOLS = mapOf("℃" to "C", "°C" to "C", "℉" to "F", "°F" to "F")

    private val ESCAPES = mapOf(
        'µ' to "\\mu ", // micro sign, as typed
        'μ' to "\\mu ", // greek small mu, as Indriya prints it
        'Ω' to "\\Omega ", // greek capital omega, as typed
        'Ω' to "\\Omega ", // ohm sign
        '°' to "^\\circ ",
    )
}
