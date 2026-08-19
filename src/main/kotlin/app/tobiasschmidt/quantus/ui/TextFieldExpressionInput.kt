package app.tobiasschmidt.quantus.ui

import javafx.scene.Node
import javafx.scene.control.TextField

/**
 * The bullet-1 placeholder editor: a plain LaTeX-source [TextField]. Bullet 2 replaces it with
 * [MathFieldView] (a WYSIWYG MathLive editor in a WebView); both implement [ExpressionInput] so
 * that swap touches nothing else in [MainView].
 */
class TextFieldExpressionInput : ExpressionInput {
    private val textField = TextField().apply {
        id = "expr-input"
        promptText = "z. B. F = 2kg \\cdot 10\\frac{m}{s^2}"
    }

    override val node: Node get() = textField

    override fun setOnEnter(handler: (String) -> Unit) {
        textField.setOnAction { handler(textField.text) }
    }

    override fun setLatex(latex: String) {
        textField.text = latex
        textField.positionCaret(latex.length)
    }

    override fun clear() {
        textField.clear()
    }

    override fun focus() {
        textField.requestFocus()
    }
}
