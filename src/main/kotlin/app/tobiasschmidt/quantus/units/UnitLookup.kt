package app.tobiasschmidt.quantus.units

import tech.units.indriya.format.SimpleUnitFormat
import java.util.Optional
import java.util.concurrent.ConcurrentHashMap
import javax.measure.Unit

/**
 * Resolves a bare unit symbol — the table behind F2's uniqueness rule.
 *
 * Indriya's [SimpleUnitFormat] already does exactly what the spec asks for, prefix
 * decomposition included, and it decides the known ambiguities the same way the spec does
 * (phase 0, spike 2): `mN` is millinewton and never metre·newton, `min` is the minute and
 * never m·i·n. So the lookup delegates instead of rebuilding prefix parsing.
 *
 * Only plain symbols are accepted. Anything containing operators, digits or whitespace is
 * rejected up front — `SimpleUnitFormat` would happily parse `m/s`, but composition is the
 * parser's job, not the lookup's.
 */
object UnitLookup {

    private val cache = ConcurrentHashMap<String, Optional<Unit<*>>>()

    /** The unit for [symbol], or `null` if it is not a known unit. */
    fun find(symbol: String): Unit<*>? =
        cache.computeIfAbsent(symbol) { candidate ->
            Optional.ofNullable(
                if (!isPlainSymbol(candidate)) {
                    null
                } else {
                    runCatching { SimpleUnitFormat.getInstance().parse(candidate) as Unit<*> }.getOrNull()
                },
            )
        }.orElse(null)

    private fun isPlainSymbol(symbol: String): Boolean =
        symbol.isNotEmpty() && symbol.all { it.isLetter() || it == '°' }
}
