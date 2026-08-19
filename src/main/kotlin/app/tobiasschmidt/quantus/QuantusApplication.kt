package app.tobiasschmidt.quantus

import app.tobiasschmidt.quantus.ui.Fonts
import app.tobiasschmidt.quantus.ui.MainView
import javafx.application.Application
import javafx.scene.Scene
import javafx.stage.Stage

/** Entry point — Phase 5: JavaFX-Grundgerüst wired to the [app.tobiasschmidt.quantus.eval.EvalFacade]. */
class QuantusApplication : Application() {

    override fun start(stage: Stage) {
        Fonts.install()

        val root = MainView().build()
        val scene = Scene(root, 1080.0, 720.0)
        scene.stylesheets.add(requireNotNull(javaClass.getResource("/css/quantus.css")).toExternalForm())

        stage.title = "Quantus"
        stage.scene = scene
        stage.show()
    }
}

fun main(args: Array<String>) {
    Application.launch(QuantusApplication::class.java, *args)
}
