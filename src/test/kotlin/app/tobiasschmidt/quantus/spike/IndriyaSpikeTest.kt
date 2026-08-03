package app.tobiasschmidt.quantus.spike

import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test
import tech.units.indriya.format.SimpleUnitFormat
import tech.units.indriya.quantity.Quantities
import tech.units.indriya.unit.Units
import java.math.BigDecimal
import javax.measure.MetricPrefix
import javax.measure.Quantity
import javax.measure.Unit
import javax.measure.quantity.Force
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

/**
 * Phase 0 / Spike 2: Indriya-Unit-Arithmetik.
 *
 * Leitfrage der Roadmap: kg × m/s² — wird daraus automatisch N?
 * Antwort: nein. Details und Konsequenzen in docs/phase0-findings.md.
 */
class IndriyaSpikeTest {

    private val format: SimpleUnitFormat = SimpleUnitFormat.getInstance()

    private fun report(label: String, value: Any?) = println("  [indriya] $label = $value")

    /**
     * Die JSR-385-Generics (`Unit<Q : Quantity<Q>>`) blockieren jede Umrechnung zwischen
     * zwei `Unit<*>`. PhysicalQuantity wird intern ebenfalls ungetypt arbeiten müssen —
     * die Dimensionsprüfung passiert bewusst zur Laufzeit, nicht im Typsystem.
     */
    @Suppress("UNCHECKED_CAST")
    private fun factor(from: Unit<*>, to: Unit<*>): Double =
        (from as Unit<Nothing>).getConverterTo(to as Unit<Nothing>).convert(1.0)

    @Test
    @DisplayName("kg × m/s² ergibt kg·m/s², NICHT automatisch N")
    fun productIsNotAutomaticallyNamed() {
        val product: Unit<*> = Units.KILOGRAM.multiply(Units.METRE_PER_SQUARE_SECOND)
        report("Produkt", format.format(product))
        report("Dimension Produkt", product.dimension)
        report("Dimension Newton", Units.NEWTON.dimension)

        assertEquals("kg·m/s²", format.format(product))
        assertFalse(product == Units.NEWTON, "Indriya benennt das Produkt nicht selbst um")

        // Die Dimensionen sind dagegen gleich — das ist der Hebel für die Benennung.
        assertEquals(Units.NEWTON.dimension, product.dimension)
        assertTrue(product.isCompatible(Units.NEWTON))
        assertEquals(mapOf("m" to 1, "kg" to 1, "s" to -2), product.baseUnits.mapKeys { format.format(it.key) })
    }

    @Test
    @DisplayName("Benennung über to(): 2 kg × 9,81 m/s² = 19,62 N")
    fun conversionToNamedUnitWorks() {
        val mass = Quantities.getQuantity(BigDecimal("2"), Units.KILOGRAM)
        val acceleration = Quantities.getQuantity(BigDecimal("9.81"), Units.METRE_PER_SQUARE_SECOND)

        val force: Quantity<*> = mass.multiply(acceleration)
        report("2 kg × 9,81 m/s²", force)
        assertEquals("19.62 kg·m/s²", force.toString())

        @Suppress("UNCHECKED_CAST")
        val asNewton = (force as Quantity<Force>).to(Units.NEWTON)
        report("umgerechnet auf N", asNewton)
        assertEquals("19.62 N", asNewton.toString())
        assertEquals(BigDecimal("19.62"), asNewton.value)
    }

    @Test
    @DisplayName("Dimension → benannte Einheit: SystemOfUnits als Seed für die Lookup-Tabelle")
    fun dimensionLookupTable() {
        val si = Units.getInstance()
        val byDimension = si.units.groupBy { it.dimension }

        val force = byDimension[Units.NEWTON.dimension]
        val speed = byDimension[Units.METRE_PER_SECOND.dimension]
        val energy = byDimension[Units.JOULE.dimension]
        report("Dimension Kraft", force?.map { format.format(it) })
        report("Dimension Geschwindigkeit", speed?.map { format.format(it) })
        report("Dimension Energie", energy?.map { format.format(it) })

        // Dimension taugt als Map-Key (equals/hashCode sind implementiert),
        // obwohl die toString-Reihenfolge der Faktoren abweicht.
        assertEquals(listOf("N"), force?.map { format.format(it) })
        assertNotNull(speed)

        // Aber: Dimensionen sind nicht eindeutig — m/s und km/h teilen sich eine,
        // ebenso J und N·m. Deshalb braucht F1/F11 eine eigene, priorisierte Tabelle.
        assertTrue(speed.size > 1, "mehrdeutig: ${speed.map { format.format(it) }}")

        val named = si.units.filter { format.format(it).length <= 3 }
        report("Kandidaten für die Lookup-Tabelle", named.size)
        report("alle SI-Einheiten", si.units.map { format.format(it) }.sorted())
    }

    @Test
    @DisplayName("Dimensionsprüfung bei + / −")
    fun incompatibleAdditionFails() {
        @Suppress("UNCHECKED_CAST")
        val metre = Quantities.getQuantity(1, Units.METRE) as Quantity<Nothing>

        @Suppress("UNCHECKED_CAST")
        val second = Quantities.getQuantity(1, Units.SECOND) as Quantity<Nothing>

        val error = runCatching { metre.add(second) }.exceptionOrNull()
        report("Fehlertyp", error?.javaClass?.name)
        report("Fehlertext", error?.message)

        assertTrue(error is javax.measure.UnconvertibleException)
        // Die Meldung ist technisch ("s is not compatible with m") — PhysicalQuantity
        // prüft in Phase 1 selbst per isCompatible() und formuliert eigene Fehler.
        assertFalse(Units.METRE.isCompatible(Units.SECOND))
    }

    @Test
    @DisplayName("Präfixe: kJ parsen, zerlegen, formatieren")
    fun prefixes() {
        val kilojoule = MetricPrefix.KILO(Units.JOULE)
        report("KILO(JOULE)", format.format(kilojoule))
        assertEquals("kJ", format.format(kilojoule))

        val parsed = format.parse("kJ")
        report("parse(\"kJ\")", parsed)
        assertTrue(parsed.isCompatible(Units.JOULE))
        assertEquals(1000.0, factor(parsed, Units.JOULE))

        report("parse(\"km/h\")", format.parse("km/h"))
        report("parse(\"mN\")", format.parse("mN"))
        report("parse(\"µm\")", format.parse("µm"))

        // mN ist Millinewton, nicht Meter·Newton — Indriya entscheidet das genauso,
        // wie die Eindeutigkeitsregel aus F2 es verlangt.
        assertEquals(0.001, factor(format.parse("mN"), Units.NEWTON))
        assertTrue(format.parse("min").isCompatible(Units.SECOND), "min ist Minute, nicht m·i·n")
    }

    @Test
    @DisplayName("Konvertierung km/h ↔ m/s (F1)")
    fun conversions() {
        val kmhToMs = Units.KILOMETRE_PER_HOUR.getConverterTo(Units.METRE_PER_SECOND)
        report("36 km/h in m/s", kmhToMs.convert(36))
        assertEquals(10.0, kmhToMs.convert(36.0))

        val quantity = Quantities.getQuantity(BigDecimal("36"), Units.KILOMETRE_PER_HOUR)
        val converted = quantity.to(Units.METRE_PER_SECOND)
        report("Quantity umgerechnet", converted)
        report("Wertklasse nach Umrechnung", converted.value.javaClass.name)
        assertEquals(0, BigDecimal("10").compareTo(BigDecimal(converted.value.toString())))

        // Achtung: aus dem BigDecimal wird bei der Umrechnung ein Integer. Indriya
        // verengt den Zahlentyp, wo es verlustfrei geht — nie blind auf BigDecimal casten.
        assertEquals("java.lang.Integer", converted.value.javaClass.name)
    }

    @Test
    @DisplayName("Physikformeln als Einheiten-Rechnung (Vorlage für Phase 1)")
    fun physicsFormulas() {
        val joule = Units.KILOGRAM
            .multiply(Units.METRE.pow(2))
            .divide(Units.SECOND.pow(2))
        report("kg·m²/s²", format.format(joule))
        assertEquals(Units.JOULE.dimension, joule.dimension)

        val volt = Units.WATT.divide(Units.AMPERE)
        report("W/A", format.format(volt))
        assertEquals(Units.VOLT.dimension, volt.dimension)

        val ohm = Units.VOLT.divide(Units.AMPERE)
        report("V/A", format.format(ohm))
        assertEquals(Units.OHM.dimension, ohm.dimension)

        val dimensionless = Units.METRE.divide(Units.METRE)
        report("m/m", format.format(dimensionless))
        assertTrue(dimensionless.isCompatible(tech.units.indriya.AbstractUnit.ONE))
    }

    @Test
    @DisplayName("Genauigkeit: Indriya rechnet mit BigDecimal weiter (passt zu F3)")
    fun bigDecimalArithmetic() {
        val a = Quantities.getQuantity(BigDecimal("0.1"), Units.METRE)
        val b = Quantities.getQuantity(BigDecimal("0.2"), Units.METRE)
        val sum = a.add(b)
        report("0,1 m + 0,2 m", sum)
        report("Wertklasse", sum.value.javaClass.name)

        assertEquals(BigDecimal("0.3"), sum.value)
        assertTrue(sum.value is BigDecimal, "kein Double-Rundungsfehler")
    }
}
