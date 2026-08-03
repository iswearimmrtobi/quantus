package app.tobiasschmidt.quantus.quantity

import app.tobiasschmidt.quantus.QuantusException
import app.tobiasschmidt.quantus.cas.CasEngine
import app.tobiasschmidt.quantus.cas.exactExpr
import app.tobiasschmidt.quantus.units.UnitConversions
import app.tobiasschmidt.quantus.units.UnitRegistry
import org.matheclipse.core.expression.F
import org.matheclipse.core.interfaces.IExpr
import org.matheclipse.core.interfaces.IRational
import tech.units.indriya.AbstractUnit
import java.math.BigDecimal
import javax.measure.Dimension
import javax.measure.Unit

/**
 * A physical quantity: numeric value **and** unit — the mediation layer between Symja and
 * Indriya described in the spec.
 *
 * - **Symja** ([value]) does everything numeric and symbolic. Values stay exact (rationals,
 *   π, roots); decimal digits only appear at the output boundary via [CasEngine].
 * - **Indriya** ([unit]) keeps the dimensional books: unit arithmetic for ×/÷, compatibility
 *   checks for +/−.
 *
 * The class is immutable; every operation returns a new instance.
 *
 * Internally everything is untyped over `Unit<*>`: the JSR-385 generics
 * (`Unit<Q : Quantity<Q>>`) forbid converting between two `Unit<*>`, and the dimension check
 * is meant to fail at runtime with a readable message (F1) rather than in the type system.
 *
 * Symbolic arithmetic runs through `F.eval`, i.e. through the thread-local `EvalEngine` —
 * the same one a [CasEngine] created on this thread configures.
 */
class PhysicalQuantity private constructor(
    val value: IExpr,
    val unit: Unit<*>,
) {

    val dimension: Dimension get() = unit.dimension

    val isDimensionless: Boolean get() = unit.isCompatible(AbstractUnit.ONE)

    /** F11 — informational quantity name (`m/s` → "velocity"), if known. */
    val quantityName: String? get() = UnitRegistry.quantityName(dimension)

    operator fun plus(other: PhysicalQuantity): PhysicalQuantity =
        PhysicalQuantity(F.eval(F.Plus(value, other.valueIn(unit, "added", this))), unit)

    operator fun minus(other: PhysicalQuantity): PhysicalQuantity =
        PhysicalQuantity(F.eval(F.Subtract(value, other.valueIn(unit, "subtracted", this))), unit)

    operator fun times(other: PhysicalQuantity): PhysicalQuantity =
        PhysicalQuantity(F.eval(F.Times(value, other.value)), unit.multiply(other.unit))

    operator fun div(other: PhysicalQuantity): PhysicalQuantity =
        PhysicalQuantity(F.eval(F.Divide(value, other.value)), unit.divide(other.unit))

    operator fun unaryMinus(): PhysicalQuantity = PhysicalQuantity(F.eval(F.Negate(value)), unit)

    fun pow(exponent: Int): PhysicalQuantity =
        PhysicalQuantity(F.eval(F.Power(value, F.ZZ(exponent.toLong()))), unit.pow(exponent))

    /**
     * Raising to an arbitrary power.
     *
     * The exponent must be dimensionless. If the base carries a unit, the exponent must also
     * be rational — only then can the unit be raised along with it (`m²` to the ½ → `m`). A
     * dimensionless base may be raised to anything.
     */
    fun pow(exponent: PhysicalQuantity): PhysicalQuantity {
        if (!exponent.isDimensionless) {
            throw DimensionMismatchException(
                "The exponent must be dimensionless but carries the unit ${UnitRegistry.describe(exponent.unit)}.",
            )
        }
        val poweredValue = F.eval(F.Power(value, exponent.value))
        if (isDimensionless) return PhysicalQuantity(poweredValue, AbstractUnit.ONE)

        val rational = exponent.value as? IRational ?: throw QuantusException(
            "A quantity with a unit can only be raised to a rational power, but the exponent " +
                "here is '${exponent.value}'.",
        )
        return PhysicalQuantity(poweredValue, unit.raisedTo(rational))
    }

    /** Square root of both value and unit (`m²` → `m`). */
    fun sqrt(): PhysicalQuantity = root(2)

    /** [n]-th root of both value and unit. */
    fun root(n: Int): PhysicalQuantity {
        require(n > 0) { "root exponent must be positive, was: $n" }
        return pow(dimensionless(F.QQ(1L, n.toLong())))
    }

    /** Explicit conversion to a target unit (F1: km/h ↔ m/s, prefixes). */
    fun to(target: Unit<*>): PhysicalQuantity {
        if (!unit.isCompatible(target)) {
            throw DimensionMismatchException(
                "Cannot convert ${UnitRegistry.describe(unit)} to ${UnitRegistry.describe(target)}.",
            )
        }
        if (unit == target) return this
        return PhysicalQuantity(UnitConversions.between(unit, target).apply(value), target)
    }

    /**
     * F1 — automatic simplification to a named unit: `kg·m/s²` → `N`. Quantities whose unit
     * the user chose (`kJ`, `km/h`) are left alone.
     */
    fun simplify(): PhysicalQuantity {
        val target = UnitRegistry.simplificationTarget(unit) ?: return this
        return to(target)
    }

    /** Numeric value with the digit count of [engine] (F3). */
    fun decimal(engine: CasEngine): BigDecimal = engine.decimal(value)

    /** The same quantity with its value evaluated numerically. */
    fun numeric(engine: CasEngine): PhysicalQuantity = PhysicalQuantity(engine.numeric(value), unit)

    /**
     * This quantity's value expressed in [target] — with a dimension check. [context] is the
     * quantity being converted to; it shows up in the error message.
     */
    private fun valueIn(target: Unit<*>, verb: String, context: PhysicalQuantity): IExpr {
        if (!unit.isCompatible(target)) {
            throw DimensionMismatchException(
                "Incompatible units: ${UnitRegistry.describe(context.unit)} and " +
                    "${UnitRegistry.describe(unit)} cannot be $verb.",
            )
        }
        val conversion = UnitConversions.between(unit, target)
        // Offset units have no meaningful sum: 20 °C + 5 K is 25 °C read as a temperature
        // difference and 298.15 °C read as two absolute temperatures. Rather than pick one
        // silently, say so — converting to K makes the intent explicit.
        if (!conversion.offset.isZero) {
            throw DimensionMismatchException(
                "${UnitRegistry.describe(context.unit)} and ${UnitRegistry.describe(unit)} cannot be $verb " +
                    "directly because one of them has a zero point of its own. Convert both to the same unit first.",
            )
        }
        return conversion.apply(value)
    }

    /** Unit to the power p/q — `unit.pow(p).root(q)`. */
    private fun Unit<*>.raisedTo(exponent: IRational): Unit<*> {
        val numerator = exponent.toBigNumerator().intValueExact()
        val denominator = exponent.toBigDenominator().intValueExact()
        val powered = pow(numerator)
        return if (denominator == 1) powered else powered.root(denominator)
    }

    override fun toString(): String = "$value ${UnitRegistry.format(unit)}"

    override fun equals(other: Any?): Boolean =
        this === other || (other is PhysicalQuantity && value == other.value && unit == other.unit)

    override fun hashCode(): Int = 31 * value.hashCode() + unit.hashCode()

    companion object {
        fun of(value: IExpr, unit: Unit<*>): PhysicalQuantity = PhysicalQuantity(F.eval(value), unit)

        fun of(value: Number, unit: Unit<*>): PhysicalQuantity = of(exactExpr(value), unit)

        /** A plain number without a unit — e.g. the ½ in `E = ½mv²`. */
        fun dimensionless(value: IExpr): PhysicalQuantity = of(value, AbstractUnit.ONE)

        fun dimensionless(value: Number): PhysicalQuantity = of(exactExpr(value), AbstractUnit.ONE)
    }
}

/** Two quantities do not fit together dimensionally (F1). */
class DimensionMismatchException(message: String) : QuantusException(message)
