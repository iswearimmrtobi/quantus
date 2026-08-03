package app.tobiasschmidt.quantus.units

import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test
import tech.units.indriya.unit.Units
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertNull
import kotlin.test.assertTrue

/**
 * Phase 2: the prefix-decomposition edge cases the spec calls out as a risk — every known
 * collision case gets a test.
 */
class UnitLookupTest {

    private fun unit(symbol: String) = assertNotNull(UnitLookup.find(symbol), "'$symbol' should be a known unit")

    @Test
    @DisplayName("base and derived units resolve")
    fun plainSymbols() {
        assertEquals(Units.KILOGRAM, unit("kg"))
        assertEquals(Units.METRE, unit("m"))
        assertEquals(Units.SECOND, unit("s"))
        assertEquals(Units.NEWTON, unit("N"))
        assertEquals(Units.JOULE, unit("J"))
        assertEquals(Units.OHM, unit("Ω"))
    }

    @Test
    @DisplayName("prefixes are decomposed: kJ is kilojoule")
    fun prefixes() {
        assertTrue(unit("kJ").isCompatible(Units.JOULE))
        assertEquals(1000.0, factorTo(unit("kJ"), Units.JOULE))
        assertEquals(0.001, factorTo(unit("mm"), Units.METRE))
        assertEquals(0.000001, factorTo(unit("µm"), Units.METRE))
    }

    @Test
    @DisplayName("the ambiguous cases go the way F2 demands")
    fun ambiguities() {
        // mN is millinewton, never metre·newton — implicit unit products are illegal.
        assertTrue(unit("mN").isCompatible(Units.NEWTON))
        assertEquals(0.001, factorTo(unit("mN"), Units.NEWTON))

        // min is the minute, not m·i·n.
        assertTrue(unit("min").isCompatible(Units.SECOND))
        assertEquals(60.0, factorTo(unit("min"), Units.SECOND))

        // Pa is the pascal, not peta·year or p·a.
        assertEquals(Units.PASCAL, unit("Pa"))

        // ms is the millisecond — that is exactly why `m s` cannot mean metre·second.
        assertEquals(0.001, factorTo(unit("ms"), Units.SECOND))
    }

    @Test
    @DisplayName("everything that is not a unit stays unknown")
    fun unknownSymbols() {
        assertNull(UnitLookup.find("xyz"))
        assertNull(UnitLookup.find("e"))
        assertNull(UnitLookup.find("i"))
        assertNull(UnitLookup.find(""))
        // composition is the parser's job, so compound spellings are refused here
        assertNull(UnitLookup.find("m/s"))
        assertNull(UnitLookup.find("kg*m"))
        assertNull(UnitLookup.find("m2"))
    }

    @Suppress("UNCHECKED_CAST")
    private fun factorTo(from: javax.measure.Unit<*>, to: javax.measure.Unit<*>): Double =
        (from as javax.measure.Unit<Nothing>).getConverterTo(to as javax.measure.Unit<Nothing>).convert(1.0)
}
