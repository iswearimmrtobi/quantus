package app.tobiasschmidt.quantus.ui

import javafx.scene.Node

/**
 * The expression editor widget, abstracted so [MainView] doesn't care whether it's backed by a
 * plain [javafx.scene.control.TextField] or the MathLive [MathFieldView] (bullet 2) — both plug
 * into the same wiring for evaluate-on-enter, history replay and the "new calculation" shortcut.
 */
interface ExpressionInput {
    /** The node to place in the layout. */
    val node: Node

    /** Called with the current LaTeX value when the user submits (Enter). */
    fun setOnEnter(handler: (String) -> Unit)

    fun setLatex(latex: String)

    fun clear()

    fun focus()
}
