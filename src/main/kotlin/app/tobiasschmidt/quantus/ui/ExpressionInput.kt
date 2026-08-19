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

    /**
     * Called with a shortcut name ("steps", "history", "vars", "new", "escape") when the user
     * types one of the app-wide shortcuts while this widget has focus. Needed because a
     * focused WebView swallows key events before they reach the [javafx.scene.Scene]'s
     * accelerators (bullet 3) — [MainView] runs the same dispatch either way.
     */
    fun setOnShortcut(handler: (String) -> Unit)

    fun setLatex(latex: String)

    fun clear()

    fun focus()
}
