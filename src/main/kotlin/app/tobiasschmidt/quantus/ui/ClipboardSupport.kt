package app.tobiasschmidt.quantus.ui

import javafx.scene.input.Clipboard
import javafx.scene.input.ClipboardContent

/** F7 — one-click copy of a step's LaTeX, or the whole calculation as an `align*` block. */
object ClipboardSupport {

    fun copyText(text: String) {
        Clipboard.getSystemClipboard().setContent(ClipboardContent().apply { putString(text) })
    }
}
