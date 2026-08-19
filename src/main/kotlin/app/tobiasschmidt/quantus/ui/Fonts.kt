package app.tobiasschmidt.quantus.ui

import javafx.scene.text.Font

/**
 * Loads the bundled IBM Plex Sans / JetBrains Mono weights (spec section 4, Phase 5 bullet 1)
 * so the app doesn't depend on them being installed system-wide. [quantus.css] refers to them
 * by exact family name, so this has to run once before the scene is shown.
 */
object Fonts {

    fun install() {
        listOf(
            "/fonts/IBMPlexSans-Regular.ttf",
            "/fonts/IBMPlexSans-Medium.ttf",
            "/fonts/IBMPlexSans-SemiBold.ttf",
            "/fonts/JetBrainsMono-Regular.ttf",
            "/fonts/JetBrainsMono-Medium.ttf",
            "/fonts/JetBrainsMono-SemiBold.ttf",
        ).forEach { path ->
            val url = requireNotNull(Fonts::class.java.getResource(path)) { "missing bundled font: $path" }
            requireNotNull(Font.loadFont(url.toExternalForm(), 13.0)) { "failed to load font: $path" }
        }
    }
}
