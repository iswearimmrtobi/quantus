package app.tobiasschmidt.quantus

import javafx.application.Application
import javafx.scene.Scene
import javafx.scene.control.Label
import javafx.scene.layout.StackPane
import javafx.stage.Stage

/** Entry point shell for Phase 5 — the real UI (editor, steps, variables) lands there. */
class QuantusApplication : Application() {

    override fun start(stage: Stage) {
        stage.title = "Quantus"
        stage.scene = Scene(StackPane(Label("Quantus")), 800.0, 600.0)
        stage.show()
    }
}

fun main(args: Array<String>) {
    Application.launch(QuantusApplication::class.java, *args)
}
