package app.tobiasschmidt.quantus.ui

import javafx.application.Platform
import javafx.concurrent.Worker
import javafx.scene.Node
import javafx.scene.web.WebView
import netscape.javascript.JSObject

/**
 * The real F10 editor (bullet 2): MathLive running in a [WebView], live-rendering LaTeX as the
 * user types (WYSIWYG — the mockup's separate ".latex-preview" box becomes redundant and is
 * dropped). Per the spec's own risk note the JS<->Kotlin bridge is kept deliberately small: it
 * carries the LaTeX value on Enter, nothing else. Kotlin -> JS goes the other way for history
 * replay and "new calculation" (`qSetValue`/`qFocus`, defined in mathfield.html).
 */
class MathFieldView : ExpressionInput {
    private val webView = WebView().apply {
        isContextMenuEnabled = false
        // WebView has no intrinsic content size (it falls back to an 800x600 default) — pin it
        // to a single input-row height and let the parent stretch it horizontally.
        prefHeight = 56.0
        minHeight = 56.0
        maxHeight = 56.0
        prefWidth = 400.0
        maxWidth = Double.MAX_VALUE
    }
    private val engine = webView.engine

    private var enterHandler: (String) -> Unit = {}
    private var loaded = false
    private var pendingLatex: String? = null
    private var pendingFocus = false

    init {
        engine.loadWorker.stateProperty().addListener { _, _, state ->
            if (state == Worker.State.SUCCEEDED) {
                installBridge()
                loaded = true
                pendingLatex?.let { applyLatex(it) }
                pendingLatex = null
                if (pendingFocus) applyFocus()
                pendingFocus = false
            }
        }
        val url = requireNotNull(MathFieldView::class.java.getResource("/webview/mathfield.html")) {
            "missing bundled resource: /webview/mathfield.html"
        }
        engine.load(url.toExternalForm())
    }

    override val node: Node get() = webView

    override fun setOnEnter(handler: (String) -> Unit) {
        enterHandler = handler
    }

    override fun setLatex(latex: String) {
        if (loaded) applyLatex(latex) else pendingLatex = latex
    }

    override fun clear() = setLatex("")

    override fun focus() {
        webView.requestFocus()
        if (loaded) applyFocus() else pendingFocus = true
    }

    private fun applyLatex(latex: String) {
        engine.executeScript("qSetValue(${jsStringLiteral(latex)})")
    }

    private fun applyFocus() {
        engine.executeScript("qFocus()")
    }

    private fun installBridge() {
        val window = engine.executeScript("window") as JSObject
        window.setMember("javaBridge", Bridge())
    }

    private fun jsStringLiteral(value: String): String {
        val escaped = value.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "\\n")
        return "\"$escaped\""
    }

    /** The JS-callable surface, installed as `window.javaBridge` once the page has loaded. */
    inner class Bridge {
        fun onEnter(latex: String) {
            Platform.runLater { enterHandler(latex) }
        }
    }
}
