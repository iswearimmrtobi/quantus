package app.tobiasschmidt.quantus.units

import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test
import tech.units.indriya.AbstractUnit
import tech.units.indriya.unit.Units
import javax.measure.MetricPrefix
import kotlin.test.assertEquals
import kotlin.test.assertNull

/** Phase 1: the dimension lookup tables behind F1 (named units) and F11 (quantity names). */
class UnitRegistryTest {

    @Test
    @DisplayName("kg·m/s² simplifies to N, kg·m²/s² to J")
    fun productUnitsGetTheirName() {
        val force = Units.KILOGRAM.multiply(Units.METRE_PER_SQUARE_SECOND)
        assertEquals(Units.NEWTON, UnitRegistry.simplificationTarget(force))

        val energy = Units.KILOGRAM.multiply(Units.METRE.pow(2)).divide(Units.SECOND.pow(2))
        assertEquals(Units.JOULE, UnitRegistry.simplificationTarget(energy))

        assertEquals(Units.VOLT, UnitRegistry.simplificationTarget(Units.WATT.divide(Units.AMPERE)))
        assertEquals(Units.OHM, UnitRegistry.simplificationTarget(Units.VOLT.divide(Units.AMPERE)))
    }

    @Test
    @DisplayName("units the user picked are left alone")
    fun chosenUnitsStay() {
        assertNull(UnitRegistry.simplificationTarget(Units.NEWTON))
        assertNull(UnitRegistry.simplificationTarget(Units.KILOMETRE_PER_HOUR))
        assertNull(UnitRegistry.simplificationTarget(MetricPrefix.KILO(Units.JOULE)))
        // m/s has no name of its own — nothing to simplify to.
        assertNull(UnitRegistry.simplificationTarget(Units.METRE.divide(Units.SECOND)))
    }

    @Test
    @DisplayName("a dimensionless product collapses to 1")
    fun dimensionlessProductCollapses() {
        val ratio = Units.JOULE.divide(Units.NEWTON.multiply(Units.METRE))
        assertEquals(AbstractUnit.ONE, UnitRegistry.simplificationTarget(ratio))
    }

    @Test
    @DisplayName("ambiguous dimensions resolve to the more common unit")
    fun ambiguousDimensionsAreDecided() {
        // 1/s is hertz and becquerel; m²/s² is gray and sievert.
        assertEquals(Units.HERTZ, UnitRegistry.preferredUnit(Units.BECQUEREL.dimension))
        assertEquals(Units.GRAY, UnitRegistry.preferredUnit(Units.SIEVERT.dimension))
        // celsius shares its dimension with kelvin — but an offset unit is no target form.
        assertEquals(Units.KELVIN, UnitRegistry.preferredUnit(Units.CELSIUS.dimension))
    }

    @Test
    @DisplayName("F11: quantity names where no named SI unit exists")
    fun quantityNames() {
        assertEquals("velocity", UnitRegistry.quantityName(Units.METRE_PER_SECOND.dimension))
        assertEquals("acceleration", UnitRegistry.quantityName(Units.METRE_PER_SQUARE_SECOND.dimension))
        assertEquals("density", UnitRegistry.quantityName(Units.KILOGRAM.divide(Units.CUBIC_METRE).dimension))
        assertEquals("force", UnitRegistry.quantityName(Units.NEWTON.dimension))
        // km/h is a velocity too — the name hangs off the dimension, not off the unit.
        assertEquals("velocity", UnitRegistry.quantityName(Units.KILOMETRE_PER_HOUR.dimension))
        assertNull(UnitRegistry.quantityName(Units.NEWTON.multiply(Units.CANDELA).dimension))
    }

    @Test
    @DisplayName("describe() feeds the error messages")
    fun describeUnits() {
        assertEquals("m (length)", UnitRegistry.describe(Units.METRE))
        assertEquals("km/h (velocity)", UnitRegistry.describe(Units.KILOMETRE_PER_HOUR))
        assertEquals("1", UnitRegistry.describe(AbstractUnit.ONE))
    }
}
