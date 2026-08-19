package app.tobiasschmidt.quantus.ui

import app.tobiasschmidt.quantus.eval.EvalFacade
import app.tobiasschmidt.quantus.eval.EvaluationResult
import app.tobiasschmidt.quantus.eval.SessionId
import app.tobiasschmidt.quantus.eval.Step
import app.tobiasschmidt.quantus.eval.StepKind
import app.tobiasschmidt.quantus.eval.VariableDefinition
import javafx.geometry.Insets
import javafx.geometry.Pos
import javafx.scene.control.Button
import javafx.scene.control.Label
import javafx.scene.control.ListCell
import javafx.scene.control.ListView
import javafx.scene.control.ScrollPane
import javafx.scene.layout.BorderPane
import javafx.scene.layout.HBox
import javafx.scene.layout.Priority
import javafx.scene.layout.Region
import javafx.scene.layout.StackPane
import javafx.scene.layout.VBox
import javafx.scene.shape.Circle

/** Which tab the side panel currently shows (rail buttons + panel tabs, mockup section 5). */
enum class PanelTab { HISTORY, VARIABLES }

/**
 * Builds and wires the whole Phase-5 UI: the layout follows `quantus-ui-v2.html` (dark theme,
 * topbar / workspace / side rail+panel / statusbar), the behaviour follows [EvalFacade] — this
 * class is the only caller of the eval facade in the app (spec section 5, N2a: no REST/JSON,
 * direct Kotlin calls).
 */
class MainView {

    private val facade = EvalFacade()
    private val sessionId = SessionId.random()
    private val history = mutableListOf<HistoryEntry>()

    private val exprInput: ExpressionInput = MathFieldView()

    private val resultValueLabel = Label("–").apply { styleClass.add("result-value") }
    private val resultUnitLabel = Label().apply { styleClass.add("result-unit") }
    private val stepsToggle = Button("Rechenweg ⌄").apply {
        styleClass.add("steps-toggle")
        tooltip = javafx.scene.control.Tooltip("Rechenweg ein-/ausblenden (Strg+J)")
    }
    private val stepsBox = VBox().apply { styleClass.add("steps-panel") }

    private val railHistoryBtn = Button("⏱").apply { styleClass.add("rail-btn") }
    private val railVarsBtn = Button("x₁").apply { styleClass.add("rail-btn") }

    private val sidePanel = VBox().apply { styleClass.add("side-panel") }
    private val historyTab = Button("Verlauf").apply { styleClass.addAll("panel-tab", "active") }
    private val varsTab = Button("Variablen").apply { styleClass.add("panel-tab") }
    private val historyList = ListView<HistoryEntry>()
    private val varsList = ListView<VariableDefinition>()

    private val statusText = Label("bereit").apply { styleClass.add("statusbar-text") }
    private val statusDot = Circle(3.0).apply { styleClass.add("status-dot") }
    private val footerCounts = Label().apply { styleClass.add("statusbar-text") }

    private var currentTab = PanelTab.HISTORY

    fun build(): Region {
        val root = BorderPane().apply { styleClass.add("root") }
        root.top = buildTopbar()
        root.center = buildCenter()
        root.bottom = buildStatusbar()

        exprInput.setOnEnter(::evaluate)
        exprInput.setOnShortcut(::handleShortcut)
        stepsToggle.setOnAction { toggleSteps() }
        railHistoryBtn.setOnAction { togglePanel(PanelTab.HISTORY) }
        railVarsBtn.setOnAction { togglePanel(PanelTab.VARIABLES) }
        historyTab.setOnAction { openPanel(PanelTab.HISTORY) }
        varsTab.setOnAction { openPanel(PanelTab.VARIABLES) }
        historyList.setOnMouseClicked {
            historyList.selectionModel.selectedItem?.let { entry -> exprInput.setLatex(entry.inputLatex); exprInput.focus() }
        }

        showEmptyResult()
        refreshHistory()
        refreshVariables()
        sidePanel.isVisible = false
        sidePanel.isManaged = false

        exprInput.focus()
        return root
    }

    fun focusInput() = exprInput.focus()

    /**
     * Dispatches an app-wide shortcut by name — bullet 3. Called both from [QuantusApplication]'s
     * [javafx.scene.Scene] accelerators (focus outside the editor) and, via
     * [ExpressionInput.setOnShortcut], from inside the WebView, which would otherwise swallow
     * the key event before it reaches the scene.
     */
    fun handleShortcut(name: String) {
        when (name) {
            "steps" -> toggleSteps()
            "history" -> togglePanel(PanelTab.HISTORY)
            "vars" -> togglePanel(PanelTab.VARIABLES)
            "new" -> newCalculation()
            "escape" -> closePanel()
        }
    }

    private fun newCalculation() {
        exprInput.clear()
        showEmptyResult()
        closePanel()
        exprInput.focus()
    }

    // ---------- topbar ----------

    private fun buildTopbar(): HBox {
        val wordmark = Label("QUANTU").apply { styleClass.addAll("wordmark") }
        val wordmarkS = Label("S").apply { styleClass.addAll("wordmark", "wordmark-accent") }
        val wordmarkBox = HBox(wordmark, wordmarkS)

        val spacer1 = Region().apply { HBox.setHgrow(this, Priority.ALWAYS) }
        val spacer2 = Region().apply { HBox.setHgrow(this, Priority.ALWAYS) }

        val precisionTag = Label("50 sig").apply { styleClass.add("precision-tag") }
        val status = HBox(6.0, statusDot, statusText).apply { alignment = Pos.CENTER_LEFT }
        val right = HBox(12.0, precisionTag, status).apply { alignment = Pos.CENTER_RIGHT }

        return HBox(wordmarkBox, spacer1, spacer2, right).apply {
            styleClass.add("topbar")
            alignment = Pos.CENTER_LEFT
        }
    }

    // ---------- center: workspace + side rail + side panel ----------

    private fun buildCenter(): StackPane {
        val workspace = buildWorkspace()
        val rail = buildSideRail()
        val overlay = HBox(sidePanel, rail)
        StackPane.setAlignment(overlay, Pos.CENTER_RIGHT)
        return StackPane(workspace, overlay).apply { styleClass.add("workspace") }
    }

    private fun buildWorkspace(): ScrollPane {
        val inputRow = StackPane(exprInput.node).apply { styleClass.add("input-row") }

        val resultEq = Label("=").apply { styleClass.add("result-eq") }
        resultUnitLabel.isVisible = false
        resultUnitLabel.isManaged = false
        val resultMain = HBox(10.0, resultEq, resultValueLabel, resultUnitLabel).apply { alignment = Pos.BASELINE_LEFT }
        val resultSpacer = Region().apply { HBox.setHgrow(this, Priority.ALWAYS) }
        val resultRow = HBox(resultMain, resultSpacer, stepsToggle).apply {
            alignment = Pos.CENTER_LEFT
            padding = Insets(22.0, 4.0, 0.0, 4.0)
        }

        stepsBox.isVisible = false
        stepsBox.isManaged = false

        val calcPanel = VBox(inputRow, resultRow, stepsBox).apply {
            styleClass.add("calc-panel")
            spacing = 0.0
            maxWidth = 720.0
        }
        VBox.setMargin(resultRow, Insets(0.0))
        VBox.setMargin(stepsBox, Insets(16.0, 0.0, 0.0, 0.0))

        val centeredPanel = StackPane(calcPanel).apply {
            padding = Insets(120.0, 0.0, 40.0, 0.0)
            alignment = Pos.TOP_CENTER
        }

        return ScrollPane(centeredPanel).apply {
            isFitToWidth = true
            styleClass.add("workspace")
            style = "-fx-background-color: transparent;"
        }
    }

    private fun buildSideRail(): VBox {
        railHistoryBtn.tooltip = javafx.scene.control.Tooltip("Verlauf (Strg+H)")
        railVarsBtn.tooltip = javafx.scene.control.Tooltip("Variablen (Strg+,)")
        return VBox(railHistoryBtn, railVarsBtn).apply { styleClass.add("side-rail") }
    }

    // ---------- side panel ----------

    init {
        val tabs = HBox(historyTab, varsTab).apply {
            styleClass.add("panel-tabs")
            HBox.setHgrow(historyTab, Priority.ALWAYS)
            HBox.setHgrow(varsTab, Priority.ALWAYS)
            historyTab.maxWidth = Double.MAX_VALUE
            varsTab.maxWidth = Double.MAX_VALUE
        }

        historyList.styleClass.add("panel-body")
        historyList.setCellFactory { historyCell() }
        historyList.placeholder = Label("Noch keine Rechnungen in dieser Sitzung.").apply { styleClass.add("empty-hint") }

        varsList.styleClass.add("panel-body")
        varsList.setCellFactory { variableCell() }
        varsList.placeholder = Label("Noch keine Variablen definiert.").apply { styleClass.add("empty-hint") }

        VBox.setVgrow(historyList, Priority.ALWAYS)
        VBox.setVgrow(varsList, Priority.ALWAYS)

        sidePanel.children.addAll(tabs, historyList)
    }

    private fun historyCell(): ListCell<HistoryEntry> = object : ListCell<HistoryEntry>() {
        override fun updateItem(item: HistoryEntry?, empty: Boolean) {
            super.updateItem(item, empty)
            styleClass.removeAll("hist-item")
            if (empty || item == null) {
                graphic = null
                return
            }
            styleClass.add("hist-item")
            val latex = Label(item.inputLatex).apply { styleClass.add("hist-latex") }
            val time = Label(item.relativeTime()).apply { styleClass.add("hist-meta") }
            val value = Label("${item.resultValue} ${item.resultUnit}".trim()).apply { styleClass.addAll("hist-meta", "hist-meta-val") }
            val metaSpacer = Region().apply { HBox.setHgrow(this, Priority.ALWAYS) }
            val meta = HBox(time, metaSpacer, value)
            graphic = VBox(4.0, latex, meta)
        }
    }

    private fun variableCell(): ListCell<VariableDefinition> = object : ListCell<VariableDefinition>() {
        override fun updateItem(item: VariableDefinition?, empty: Boolean) {
            super.updateItem(item, empty)
            styleClass.removeAll("var-item")
            if (empty || item == null) {
                graphic = null
                return
            }
            styleClass.add("var-item")
            val name = Label(item.key.toString()).apply { styleClass.add("var-name") }
            val spacer = Region().apply { HBox.setHgrow(this, Priority.ALWAYS) }
            val value = Label(item.value.toString()).apply { styleClass.add("var-val") }
            graphic = HBox(name, spacer, value).apply { alignment = Pos.CENTER_LEFT }
        }
    }

    private fun openPanel(tab: PanelTab) {
        currentTab = tab
        sidePanel.isVisible = true
        sidePanel.isManaged = true
        historyTab.styleClass.setAll(if (tab == PanelTab.HISTORY) listOf("panel-tab", "active") else listOf("panel-tab"))
        varsTab.styleClass.setAll(if (tab == PanelTab.VARIABLES) listOf("panel-tab", "active") else listOf("panel-tab"))
        railHistoryBtn.styleClass.setAll(if (tab == PanelTab.HISTORY) listOf("rail-btn", "active") else listOf("rail-btn"))
        railVarsBtn.styleClass.setAll(if (tab == PanelTab.VARIABLES) listOf("rail-btn", "active") else listOf("rail-btn"))
        sidePanel.children.setAll(sidePanel.children[0], if (tab == PanelTab.HISTORY) historyList else varsList)
        if (tab == PanelTab.VARIABLES) refreshVariables()
    }

    fun togglePanel(tab: PanelTab) {
        if (sidePanel.isVisible && currentTab == tab) {
            closePanel()
        } else {
            openPanel(tab)
        }
    }

    fun closePanel() {
        sidePanel.isVisible = false
        sidePanel.isManaged = false
        railHistoryBtn.styleClass.setAll("rail-btn")
        railVarsBtn.styleClass.setAll("rail-btn")
    }

    // ---------- statusbar ----------

    private fun buildStatusbar(): HBox {
        val left = Label("Symja · Indriya — Session aktiv").apply { styleClass.add("statusbar-text") }
        val spacer = Region().apply { HBox.setHgrow(this, Priority.ALWAYS) }
        updateFooterCounts()
        return HBox(left, spacer, footerCounts).apply {
            styleClass.add("statusbar")
            alignment = Pos.CENTER_LEFT
        }
    }

    private fun updateFooterCounts() {
        val vars = facade.getVariables(sessionId).size
        footerCounts.text = "$vars Variable${if (vars == 1) "" else "n"} · ${history.size} Einträge im Verlauf"
    }

    // ---------- evaluation ----------

    fun evaluate(latex: String) {
        if (latex.isBlank()) return
        val result = facade.evaluate(latex, sessionId)
        if (result.error != null) {
            showError(result.error)
        } else {
            showResult(result)
            history.add(0, HistoryEntry(latex, result.value, result.unit))
            refreshHistory()
        }
        refreshVariables()
        updateFooterCounts()
    }

    private fun showEmptyResult() {
        resultValueLabel.text = "–"
        resultValueLabel.styleClass.setAll("result-value")
        resultUnitLabel.isVisible = false
        resultUnitLabel.isManaged = false
        statusText.text = "bereit"
        statusDot.styleClass.setAll("status-dot")
        setStepsVisible(false)
        stepsBox.children.clear()
    }

    private fun showResult(result: EvaluationResult) {
        resultValueLabel.text = result.value
        resultValueLabel.styleClass.setAll("result-value")
        if (result.unit != "1" && result.unit.isNotBlank()) {
            resultUnitLabel.text = result.unit
            resultUnitLabel.isVisible = true
            resultUnitLabel.isManaged = true
        } else {
            resultUnitLabel.isVisible = false
            resultUnitLabel.isManaged = false
        }
        statusText.text = "bereit"
        statusDot.styleClass.setAll("status-dot")
        renderSteps(result.steps)
    }

    private fun showError(message: String) {
        resultValueLabel.text = message
        resultValueLabel.styleClass.setAll("result-value", "error")
        resultUnitLabel.isVisible = false
        resultUnitLabel.isManaged = false
        statusText.text = "Fehler"
        statusDot.styleClass.setAll("status-dot", "error")
        setStepsVisible(false)
        stepsBox.children.clear()
    }

    private fun renderSteps(steps: List<Step>) {
        stepsBox.children.setAll(steps.map(::stepRow))
        setStepsVisible(stepsBox.isVisible)
    }

    private fun stepRow(step: Step): HBox {
        val labelText = when (step.kind) {
            StepKind.SYMBOLIC -> "SYMBOLISCH"
            StepKind.SUBSTITUTED -> "EINGESETZT"
            StepKind.RESULT -> "ERGEBNIS"
        }
        val label = Label(labelText).apply {
            styleClass.add("step-label")
            if (step.kind == StepKind.RESULT) styleClass.add("result-label")
        }
        val latex = Label(step.latex).apply { styleClass.add("step-latex") }
        val spacer = Region().apply { HBox.setHgrow(this, Priority.ALWAYS) }
        return HBox(label, latex, spacer).apply { styleClass.add("step-row") }
    }

    private fun toggleSteps() = setStepsVisible(!stepsBox.isVisible)

    private fun setStepsVisible(visible: Boolean) {
        val hasSteps = stepsBox.children.isNotEmpty()
        stepsBox.isVisible = visible && hasSteps
        stepsBox.isManaged = visible && hasSteps
        stepsToggle.text = if (stepsBox.isVisible) "Rechenweg ⌃" else "Rechenweg ⌄"
    }

    // ---------- panel data ----------

    private fun refreshHistory() {
        historyList.items.setAll(history)
    }

    private fun refreshVariables() {
        varsList.items.setAll(facade.getVariables(sessionId))
    }
}
