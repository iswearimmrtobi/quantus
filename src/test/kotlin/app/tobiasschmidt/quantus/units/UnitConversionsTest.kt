package app.tobiasschmidt.quantus.units

import app.tobiasschmidt.quantus.cas.CasEngine
import app.tobiasschmidt.quantus.cas.exactExpr
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test
import java.math.BigDecimal
import java.math.MathContext
import javax.measure.MetricPrefix
import tech.units.indriya.unit.Units
import kotlin.test.assertEquals
import kotlin.test.assertTrue

/** Phase 1: unit conversions must not cost precision (F3). */
class UnitConversionsTest {

    private val engine = CasEngine()

    @Test
    @DisplayName("conversion factors stay exact fractions")
    fun factorsAreExact() {
        // 5/18 is periodic in decimal — Indriya's own converter would cut it off at 34 digits.
        val kmhToMs = UnitConversions.between(Units.KILOMETRE_PER_HOUR, Units.METRE_PER_SECOND)
        assertEquals("5/18", kmhToMs.factor.toString())
        assertTrue(kmhToMs.offset.isZero)

        val oneKmh = engine.decimal(kmhToMs.apply(exactExpr(1)))
        assertEquals(0, BigDecimal.ONE.divide(BigDecimal("3.6"), MathContext(50)).compareTo(oneKmh))
        assertEquals(50, oneKmh.precision())
    }

    @Test
    @DisplayName("prefixes are powers of ten, not doubles")
    fun prefixFactors() {
        assertEquals("1000", UnitConversions.between(MetricPrefix.KILO(Units.JOULE), Units.JOULE).factor.toString())
        assertEquals("1/1000", UnitConversions.between(MetricPrefix.MILLI(Units.NEWTON), Units.NEWTON).factor.toString())
        assertEquals("1/1000000", UnitConversions.between(MetricPrefix.MICRO(Units.METRE), Units.METRE).factor.toString())
    }

    @Test
    @DisplayName("celsius carries an offset")
    fun affineConversion() {
        val toKelvin = UnitConversions.between(Units.CELSIUS, Units.KELVIN)
        assertEquals("1", toKelvin.factor.toString())
        assertEquals("5463/20", toKelvin.offset.toString())
        assertEquals(0, BigDecimal("298.15").compareTo(engine.decimal(toKelvin.apply(exactExpr(25)))))
    }

    @Test
    @DisplayName("identical units convert to the identity")
    fun identity() {
        assertTrue(UnitConversions.between(Units.NEWTON, Units.NEWTON).isIdentity)
        val expr = exactExpr(BigDecimal("1.5"))
        assertEquals(expr, UnitConversions.between(Units.NEWTON, Units.NEWTON).apply(expr))
    }

    @Test
    @DisplayName("the folded affine map agrees with Indriya on every unit pair")
    fun foldedConversionMatchesIndriya() {
        val pairs = listOf(
            Units.KILOMETRE_PER_HOUR to Units.METRE_PER_SECOND,
            Units.METRE_PER_SECOND to Units.KILOMETRE_PER_HOUR,
            Units.CELSIUS to Units.KELVIN,
            Units.KELVIN to Units.CELSIUS,
            Units.HOUR to Units.SECOND,
            Units.LITRE to Units.CUBIC_METRE,
            MetricPrefix.KILO(Units.JOULE) to Units.JOULE,
            Units.GRAM to Units.KILOGRAM,
            Units.KILOGRAM.multiply(Units.METRE_PER_SQUARE_SECOND) to Units.NEWTON,
            // scale *and* offset in one converter: catches a wrong step order in the fold
            Units.CELSIUS.multiply(2) to Units.KELVIN,
            MetricPrefix.MILLI(Units.CELSIUS) to Units.KELVIN,
        )

        for ((from, to) in pairs) {
            val conversion = UnitConversions.between(from, to)
            for (probe in listOf("0", "1", "7.5", "-12.25")) {
                val expected = from.getConverterToAny(to).convert(BigDecimal(probe).toDouble())
                val actual = engine.decimal(conversion.apply(exactExpr(BigDecimal(probe)))).toDouble()
                assertEquals(
                    expected,
                    actual,
                    1e-9,
                    "$probe ${UnitRegistry.format(from)} -> ${UnitRegistry.format(to)}",
                )
            }
        }
    }
}
