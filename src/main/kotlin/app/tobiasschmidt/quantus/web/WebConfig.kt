package app.tobiasschmidt.quantus.web

import org.springframework.beans.factory.annotation.Value
import org.springframework.context.annotation.Configuration
import org.springframework.web.servlet.config.annotation.CorsRegistry
import org.springframework.web.servlet.config.annotation.WebMvcConfigurer

/**
 * CORS for the two ways the renderer reaches the backend: the Vite dev server during
 * development, and the packaged app, whose `file://` page sends `Origin: null`.
 *
 * The list is deliberately short and configurable (`quantus.cors.allowed-origins`) — the
 * backend listens on the loopback interface, and nothing beyond the app should be talking
 * to it.
 */
@Configuration
class WebConfig(
    @Value("\${quantus.cors.allowed-origins}") private val allowedOrigins: Array<String>,
) : WebMvcConfigurer {

    override fun addCorsMappings(registry: CorsRegistry) {
        registry.addMapping("/api/**")
            .allowedOrigins(*allowedOrigins)
            .allowedMethods("GET", "POST", "DELETE")
    }
}
