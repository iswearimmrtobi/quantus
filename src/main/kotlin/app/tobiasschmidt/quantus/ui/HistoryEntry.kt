package app.tobiasschmidt.quantus.ui

import java.time.Duration
import java.time.Instant

/**
 * One evaluated calculation, kept in memory for the session's history panel. Not persisted —
 * that is the SQLite-backed history of Phase 6; this is purely the in-session list the
 * Phase 5 mockup's "Verlauf" tab shows.
 */
data class HistoryEntry(
    val inputLatex: String,
    val resultValue: String,
    val resultUnit: String,
    val at: Instant = Instant.now(),
) {
    /** German relative time, coarse on purpose ("vor 4 Min") — matches the mockup's style. */
    fun relativeTime(now: Instant = Instant.now()): String {
        val seconds = Duration.between(at, now).seconds.coerceAtLeast(0)
        return when {
            seconds < 60 -> "gerade eben"
            seconds < 3600 -> "vor ${seconds / 60} Min"
            seconds < 86400 -> "vor ${seconds / 3600} Std"
            else -> "vor ${seconds / 86400} Tg"
        }
    }
}
