package app.tobiasschmidt.quantus

/**
 * Base class for all domain errors whose message is safe to show to the user.
 * Anything not derived from this class is an internal defect.
 */
open class QuantusException(message: String, cause: Throwable? = null) : RuntimeException(message, cause)
