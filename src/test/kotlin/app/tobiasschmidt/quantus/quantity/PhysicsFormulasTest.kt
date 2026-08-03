package app.tobiasschmidt.quantus.quantity

import app.tobiasschmidt.quantus.cas.CasEngine
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test
import org.matheclipse.core.expression.F
import tech.units.indriya.unit.Units
import java.math.BigDecimal
import javax.measure.MetricPrefix
import javax.measure.Unit
import kotlin.test.assertEquals
import kotlin.test.assertTrue

/**
 * Phase 1: the textbook formulas as test cases. Every one of them is a small integration
 * test of value arithmetic (Symja), dimensional bookkeeping (Indriya) and the naming of the
 * result (F1).
 */
class PhysicsFormulasTest {

    private val engine = CasEngine()

    private fun quantity(value: String, unit: Unit<*>) = PhysicalQuantity.of(BigDecimal(value), unit)

    private fun assertResult(expected: String, unit: Unit<*>, actual: PhysicalQuantity) {
        assertEquals(unit, actual.unit, "unit of $actual")
        assertEquals(0, BigDecimal(expected).compareTo(actual.decimal(engine)), "value of $actual")
    }

    @Test
    @DisplayName("F = m · a")
    fun force() {
        val mass = quantity("2", Units.KILOGRAM)
        val acceleration = quantity("9.81", Units.METRE_PER_SQUARE_SECOND)

        assertResult("19.62", Units.NEWTON, (mass * acceleration).simplify())
    }

    @Test
    @DisplayName("E = ½ m v²")
    fun kineticEnergy() {
        val half = PhysicalQuantity.dimensionless(F.QQ(1, 2))
        val mass = quantity("2", Units.KILOGRAM)
        val velocity = quantity("3", Units.METRE_PER_SECOND)

        assertResult("9", Units.JOULE, (half * mass * velocity.pow(2)).simplify())

        // The same formula with the velocity in km/h: 36 km/h are 10 m/s, so 100 J.
        val inKmh = quantity("36", Units.KILOMETRE_PER_HOUR)
        assertResult("100", Units.JOULE, (half * mass * inKmh.pow(2)).simplify())
    }

    @Test
    @DisplayName("U = R · I")
    fun ohmsLaw() {
        val resistance = quantity("5", Units.OHM)
        val current = quantity("2", Units.AMPERE)
        val voltage = (resistance * current).simplify()

        assertResult("10", Units.VOLT, voltage)
        // and back: R = U / I
        assertResult("5", Units.OHM, (voltage / current).simplify())
    }

    @Test
    @DisplayName("p = F / A")
    fun pressure() {
        val force = quantity("100", Units.NEWTON)
        val area = quantity("2", Units.SQUARE_METRE)

        assertResult("50", Units.PASCAL, (force / area).simplify())
    }

    @Test
    @DisplayName("ρ = m / V — no named unit, but a quantity name (F11)")
    fun density() {
        val mass = quantity("1000", Units.KILOGRAM)
        val volume = quantity("2", Units.CUBIC_METRE)
        val density = (mass / volume).simplify()

        assertResult("500", Units.KILOGRAM.divide(Units.CUBIC_METRE), density)
        assertEquals("density", density.quantityName)
    }

    @Test
    @DisplayName("W = P · t — the minute is converted along the way")
    fun work() {
        val power = quantity("60", Units.WATT)
        val time = quantity("2", Units.MINUTE)

        assertResult("7200", Units.JOULE, (power * time).simplify())
    }

    @Test
    @DisplayName("E = m c² — exact, without a single rounding step")
    fun massEnergyEquivalence() {
        val mass = quantity("1", Units.KILOGRAM)
        val c = quantity("299792458", Units.METRE_PER_SECOND)
        val energy = (mass * c.pow(2)).simplify()

        assertResult("89875517873681764", Units.JOULE, energy)
        assertEquals("89875517873681764", energy.value.toString())
    }

    @Test
    @DisplayName("T = 2π √(l/g) — symbolic π and a square root, resolved to 50 digits")
    fun pendulum() {
        val length = quantity("1", Units.METRE)
        val g = quantity("9.81", Units.METRE_PER_SQUARE_SECOND)

        val period = (PhysicalQuantity.dimensionless(2) * PhysicalQuantity.dimensionless(F.Pi) *
            (length / g).sqrt()).simplify()

        assertEquals(Units.SECOND, period.unit)
        // (2·π·√(100/981)) stays exact until the very end.
        assertEquals("20/3*Pi/Sqrt(109)", period.value.toString())
        assertTrue(
            period.decimal(engine).toPlainString().startsWith("2.00606668071064744"),
            "expected the pendulum period, got ${period.decimal(engine)}",
        )
        assertEquals(50, period.decimal(engine).precision())
    }

    @Test
    @DisplayName("Q = c · m · ΔT — a compound unit that stays compound")
    fun heat() {
        val specificHeat = quantity("4182", Units.JOULE.divide(Units.KILOGRAM.multiply(Units.KELVIN)))
        val mass = quantity("0.5", Units.KILOGRAM)
        val deltaT = quantity("20", Units.KELVIN)

        val heat = (specificHeat * mass * deltaT).simplify()
        assertResult("41820", Units.JOULE, heat)
        assertEquals("specific heat capacity", specificHeat.quantityName)
    }

    @Test
    @DisplayName("results can be converted to a unit of choice: 41.82 kJ")
    fun conversionOfTheResult() {
        val heat = quantity("41820", Units.JOULE)
        assertResult("41.82", MetricPrefix.KILO(Units.JOULE), heat.to(MetricPrefix.KILO(Units.JOULE)))
    }
}
