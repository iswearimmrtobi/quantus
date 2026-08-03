package app.tobiasschmidt.quantus

import app.tobiasschmidt.quantus.cas.CasEngine
import org.springframework.boot.context.properties.ConfigurationProperties

/** Everything configurable about the calculator itself (`quantus.*` in application.yaml). */
@ConfigurationProperties(prefix = "quantus")
data class QuantusProperties(
    /** Significant digits for numeric results (F3); a request may ask for a different count. */
    val precision: Int = CasEngine.DEFAULT_PRECISION,
)
