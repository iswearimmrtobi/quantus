package app.tobiasschmidt.quantus

import app.tobiasschmidt.quantus.ui.Fonts
import app.tobiasschmidt.quantus.ui.MainView
import javafx.application.Application
import javafx.scene.Scene
import javafx.scene.input.KeyCode
import javafx.scene.input.KeyCodeCombination
import javafx.scene.input.KeyCombination
import javafx.stage.Stage

/** Entry point — Phase 5: JavaFX-Grundgerüst wired to the [app.tobiasschmidt.quantus.eval.EvalFacade]. */
class QuantusApplication : Application() {

    override fun start(stage: Stage) {
        Fonts.install()

        val view = MainView()
        val root = view.build()
        val scene = Scene(root, 1080.0, 720.0)
        scene.stylesheets.add(requireNotNull(javaClass.getResource("/css/quantus.css")).toExternalForm())
        installAccelerators(scene, view)

        stage.title = "Quantus"
        stage.scene = scene
        stage.show()
    }

    /**
     * Bullet 3 — app-wide keyboard shortcuts via JavaFX Accelerators (no separate global-shortcut
     * mechanism, unlike the old Electron design). Only covers focus *outside* the editor: a
     * focused WebView swallows these key events first, so [MainView] wires the same dispatch via
     * [app.tobiasschmidt.quantus.ui.ExpressionInput.setOnShortcut] for focus inside it.
     */
    private fun installAccelerators(scene: Scene, view: MainView) {
        val accelerators = scene.accelerators
        accelerators[KeyCodeCombination(KeyCode.J, KeyCombination.CONTROL_DOWN)] = Runnable { view.handleShortcut("steps") }
        accelerators[KeyCodeCombination(KeyCode.H, KeyCombination.CONTROL_DOWN)] = Runnable { view.handleShortcut("history") }
        accelerators[KeyCodeCombination(KeyCode.COMMA, KeyCombination.CONTROL_DOWN)] = Runnable { view.handleShortcut("vars") }
        accelerators[KeyCodeCombination(KeyCode.N, KeyCombination.CONTROL_DOWN)] = Runnable { view.handleShortcut("new") }
        accelerators[KeyCodeCombination(KeyCode.C, KeyCombination.CONTROL_DOWN, KeyCombination.SHIFT_DOWN)] =
            Runnable { view.handleShortcut("copyAll") }
        accelerators[KeyCodeCombination(KeyCode.ESCAPE)] = Runnable { view.handleShortcut("escape") }
    }
}

fun main(args: Array<String>) {
    Application.launch(QuantusApplication::class.java, *args)
}
