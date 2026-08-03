package app.tobiasschmidt.quantus.quantity

import app.tobiasschmidt.quantus.cas.CasEngine
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test
import org.matheclipse.core.expression.F
import tech.units.indriya.AbstractUnit
import tech.units.indriya.unit.Units
import java.math.BigDecimal
import javax.measure.MetricPrefix
import javax.measure.Unit
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

/** Phase 1: arithmetic on PhysicalQuantity — value via Symja, dimensions via Indriya. */
class PhysicalQuantityTest {

    private val engine = CasEngine()

    private fun assertQuantity(expected: String, unit: Unit<*>, actual: PhysicalQuantity) {
        assertEquals(unit, actual.unit, "unit of $actual")
        assertEquals(0, BigDecimal(expected).compareTo(actual.decimal(engine)), "value of $actual")
    }

    @Test
    @DisplayName("milestone: 1 kg × 1 m/s² = 1 N, at 50 digits")
    fun theMilestone() {
        val mass = PhysicalQuantity.of(1, Units.KILOGRAM)
        val acceleration = PhysicalQuantity.of(1, Units.METRE_PER_SQUARE_SECOND)

        val force = (mass * acceleration).simplify()

        assertEquals(Units.NEWTON, force.unit)
        assertEquals("force", force.quantityName)
        assertQuantity("1", Units.NEWTON, force)
        assertEquals(50, engine.precision)
    }

    @Test
    @DisplayName("addition converts the right operand into the left unit")
    fun additionConverts() {
        val sum = PhysicalQuantity.of(1, Units.KILOMETRE_PER_HOUR) + PhysicalQuantity.of(1, Units.METRE_PER_SECOND)
        assertQuantity("4.6", Units.KILOMETRE_PER_HOUR, sum)

        val difference = PhysicalQuantity.of(1, Units.METRE) - PhysicalQuantity.of(50, MetricPrefix.CENTI(Units.METRE))
        assertQuantity("0.5", Units.METRE, difference)
    }

    @Test
    @DisplayName("addition of incompatible quantities explains itself")
    fun incompatibleAddition() {
        val length = PhysicalQuantity.of(5, Units.METRE)
        val time = PhysicalQuantity.of(3, Units.SECOND)

        val error = assertFailsWith<DimensionMismatchException> { length + time }
        assertEquals("Incompatible units: m (length) and s (time) cannot be added.", error.message)

        val subtraction = assertFailsWith<DimensionMismatchException> { length - time }
        assertTrue(subtraction.message!!.endsWith("cannot be subtracted."), subtraction.message!!)
    }

    @Test
    @DisplayName("multiplication and division combine the units")
    fun multiplicationAndDivision() {
        val distance = PhysicalQuantity.of(100, Units.METRE)
        val time = PhysicalQuantity.of(8, Units.SECOND)

        val velocity = distance / time
        assertQuantity("12.5", Units.METRE.divide(Units.SECOND), velocity)
        assertEquals("velocity", velocity.quantityName)

        val area = PhysicalQuantity.of(3, Units.METRE) * PhysicalQuantity.of(4, Units.METRE)
        assertQuantity("12", Units.METRE.pow(2), area)
        assertEquals("area", area.quantityName)
    }

    @Test
    @DisplayName("powers and roots take the unit along")
    fun powersAndRoots() {
        val side = PhysicalQuantity.of(3, Units.METRE)
        assertQuantity("27", Units.METRE.pow(3), side.pow(3))

        val inverseSquare = side.pow(-2)
        assertEquals(Units.METRE.pow(-2), inverseSquare.unit)
        assertEquals("1/9", inverseSquare.value.toString())

        val area = PhysicalQuantity.of(9, Units.METRE.pow(2))
        assertQuantity("3", Units.METRE, area.sqrt())

        val volume = PhysicalQuantity.of(8, Units.METRE.pow(3))
        assertQuantity("2", Units.METRE, volume.root(3))
    }

    @Test
    @DisplayName("dimensionless values keep their symbolic exactness")
    fun dimensionlessStaysSymbolic() {
        val two = PhysicalQuantity.dimensionless(2)
        assertEquals("Sqrt(2)", two.sqrt().value.toString())
        assertTrue(two.sqrt().isDimensionless)

        // e^π — no rational exponent in sight, and that is fine without a unit.
        val exponential = PhysicalQuantity.dimensionless(F.E).pow(PhysicalQuantity.dimensionless(F.Pi))
        assertTrue(exponential.decimal(engine).toPlainString().startsWith("23.1406926327792690057290863679485473802661062426"))
    }

    @Test
    @DisplayName("a unit cannot be raised to an irrational power, and no exponent may carry a unit")
    fun invalidExponents() {
        val length = PhysicalQuantity.of(2, Units.METRE)

        val irrational = assertFailsWith<app.tobiasschmidt.quantus.QuantusException> {
            length.pow(PhysicalQuantity.dimensionless(F.Sqrt(F.C2)))
        }
        assertTrue(irrational.message!!.contains("rational power"), irrational.message!!)

        val dimensioned = assertFailsWith<DimensionMismatchException> {
            length.pow(PhysicalQuantity.of(2, Units.SECOND))
        }
        assertTrue(dimensioned.message!!.contains("must be dimensionless"), dimensioned.message!!)
    }

    @Test
    @DisplayName("explicit conversion, including the refusal to convert nonsense")
    fun explicitConversion() {
        val speed = PhysicalQuantity.of(36, Units.KILOMETRE_PER_HOUR)
        assertQuantity("10", Units.METRE_PER_SECOND, speed.to(Units.METRE_PER_SECOND))

        val energy = PhysicalQuantity.of(2, MetricPrefix.KILO(Units.JOULE))
        assertQuantity("2000", Units.JOULE, energy.to(Units.JOULE))

        val error = assertFailsWith<DimensionMismatchException> { speed.to(Units.JOULE) }
        assertEquals("Cannot convert km/h (velocity) to J (energy).", error.message)
    }

    @Test
    @DisplayName("simplify() only touches units that arithmetic produced")
    fun simplifyIsConservative() {
        val chosen = PhysicalQuantity.of(2, MetricPrefix.KILO(Units.JOULE))
        assertEquals(chosen, chosen.simplify())

        val speed = PhysicalQuantity.of(36, Units.KILOMETRE_PER_HOUR)
        assertEquals(speed, speed.simplify())

        // A quotient that cancels out completely ends up dimensionless.
        val ratio = (PhysicalQuantity.of(6, Units.METRE) / PhysicalQuantity.of(4, Units.METRE)).simplify()
        assertEquals(AbstractUnit.ONE, ratio.unit)
        assertQuantity("1.5", AbstractUnit.ONE, ratio)
    }

    @Test
    @DisplayName("values stay exact until the decimal boundary")
    fun exactUntilTheEnd() {
        val third = PhysicalQuantity.of(1, Units.METRE) / PhysicalQuantity.dimensionless(3)
        assertEquals("1/3", third.value.toString())

        val restored = (third * PhysicalQuantity.dimensionless(3)).simplify()
        assertEquals("1", restored.value.toString())
        assertQuantity("1", Units.METRE, restored)
    }

    @Test
    @DisplayName("negation keeps the unit")
    fun negation() {
        assertQuantity("-3", Units.KELVIN, -PhysicalQuantity.of(3, Units.KELVIN))
    }
}
