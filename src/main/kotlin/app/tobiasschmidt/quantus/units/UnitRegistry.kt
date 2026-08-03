package app.tobiasschmidt.quantus.units

import tech.units.indriya.AbstractUnit
import tech.units.indriya.format.SimpleUnitFormat
import tech.units.indriya.unit.ProductUnit
import tech.units.indriya.unit.Units
import javax.measure.Dimension
import javax.measure.Unit

/**
 * The two lookup tables behind F1 and F11.
 *
 * Indriya does not rename products on its own: `kg × m/s²` stays `kg·m/s²` and is
 * explicitly *not* `N` (phase 0, spike 2). The dimensions do match though, and that is the
 * lever: dimension → preferred named unit, then convert.
 *
 * Dimensions are **not unique** (m/s and km/h share one, as do Hz and Bq, or Gy and Sv).
 * Hence a hand-curated, prioritised table instead of `Units.getInstance().units`: the first
 * match in [PREFERRED_UNITS] wins.
 */
object UnitRegistry {

    /**
     * The coherent SI units that carry a name of their own, in priority order.
     *
     * Deliberately *only* named units: `m²` or `m/s` have no name of their own, so `km·km`
     * stays `km²` and `36 km/h` stays `km/h`. Simplification only happens where a real name
     * exists (F1: "kg·m/s² → N").
     *
     * Conflicts this ordering decides:
     * - `1/s`: hertz (frequency) over becquerel (activity)
     * - `m²/s²`: gray (absorbed dose) over sievert (dose equivalent)
     * - `cd`: the base unit over lumen (cd·sr, dimensionally identical)
     * - `K`: kelvin over degree celsius (celsius carries an offset, unfit as a target form)
     */
    private val PREFERRED_UNITS: List<Unit<*>> = listOf(
        // base units first
        Units.METRE,
        Units.KILOGRAM,
        Units.SECOND,
        Units.AMPERE,
        Units.KELVIN,
        Units.MOLE,
        Units.CANDELA,
        // named derived units
        Units.NEWTON,
        Units.PASCAL,
        Units.JOULE,
        Units.WATT,
        Units.COULOMB,
        Units.VOLT,
        Units.FARAD,
        Units.OHM,
        Units.SIEMENS,
        Units.WEBER,
        Units.TESLA,
        Units.HENRY,
        Units.LUX,
        Units.HERTZ,
        Units.GRAY,
        Units.KATAL,
    )

    private val preferredByDimension: Map<Dimension, Unit<*>> =
        PREFERRED_UNITS.groupBy { it.dimension }.mapValues { (_, candidates) -> candidates.first() }

    /**
     * F11 — purely informational quantity names. Complements the unit table where no named
     * SI unit exists (m/s → "velocity"), and doubles as the source for readable error
     * messages ("m (length) and s (time) cannot be added").
     *
     * Dimensions are derived from units rather than written out by hand, so no dimensional
     * algebra can go wrong here. Ambiguous dimensions (J = N·m = torque, Hz = Bq) carry the
     * more common name.
     */
    private val quantityNames: Map<Dimension, String> = buildMap {
        fun name(unit: Unit<*>, name: String) = putIfAbsent(unit.dimension, name)

        name(Units.METRE, "length")
        name(Units.SQUARE_METRE, "area")
        name(Units.CUBIC_METRE, "volume")
        name(Units.KILOGRAM, "mass")
        name(Units.SECOND, "time")
        name(Units.AMPERE, "electric current")
        name(Units.KELVIN, "temperature")
        name(Units.MOLE, "amount of substance")
        name(Units.CANDELA, "luminous intensity")

        name(Units.METRE_PER_SECOND, "velocity")
        name(Units.METRE_PER_SQUARE_SECOND, "acceleration")
        name(Units.KILOGRAM.divide(Units.CUBIC_METRE), "density")
        name(Units.KILOGRAM.multiply(Units.METRE_PER_SECOND), "momentum")
        name(Units.CUBIC_METRE.divide(Units.SECOND), "volumetric flow rate")

        name(Units.HERTZ, "frequency")
        name(Units.NEWTON, "force")
        name(Units.PASCAL, "pressure")
        name(Units.JOULE, "energy")
        name(Units.WATT, "power")
        name(Units.COULOMB, "electric charge")
        name(Units.VOLT, "voltage")
        name(Units.FARAD, "capacitance")
        name(Units.OHM, "resistance")
        name(Units.SIEMENS, "conductance")
        name(Units.WEBER, "magnetic flux")
        name(Units.TESLA, "magnetic flux density")
        name(Units.HENRY, "inductance")
        name(Units.LUX, "illuminance")
        name(Units.GRAY, "absorbed dose")
        name(Units.KATAL, "catalytic activity")

        name(Units.JOULE.divide(Units.KELVIN), "heat capacity")
        name(Units.JOULE.divide(Units.KILOGRAM.multiply(Units.KELVIN)), "specific heat capacity")
    }

    /** The preferred named unit for a dimension, or `null` if there is none. */
    fun preferredUnit(dimension: Dimension): Unit<*>? = preferredByDimension[dimension]

    /**
     * Target unit for automatic simplification (F1) — or `null` if [unit] should stay as it is.
     *
     * Only what arithmetic produced gets simplified: a [ProductUnit] such as `kg·m/s²`.
     * Units the user picked (`kJ`, `km/h`, `N`) are left alone — converting the result to a
     * requested unit ("give me kWh") is a separate, explicit operation.
     */
    fun simplificationTarget(unit: Unit<*>): Unit<*>? {
        if (unit !is ProductUnit<*>) return null
        val target = preferredByDimension[unit.dimension]
            ?: if (unit.isCompatible(AbstractUnit.ONE)) AbstractUnit.ONE else null
        return target?.takeIf { it != unit }
    }

    /** F11 — the common physical name of the quantity, purely informational. */
    fun quantityName(dimension: Dimension): String? = quantityNames[dimension]

    /** Display form of a unit, for error messages and debug output. */
    fun format(unit: Unit<*>): String =
        if (unit == AbstractUnit.ONE) "1" else SimpleUnitFormat.getInstance().format(unit)

    /** Unit plus quantity name, e.g. `m/s² (acceleration)` — for error messages. */
    fun describe(unit: Unit<*>): String {
        val name = quantityName(unit.dimension)
        return if (name == null) format(unit) else "${format(unit)} ($name)"
    }
}
