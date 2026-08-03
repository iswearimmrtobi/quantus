package app.tobiasschmidt.quantus.web

import app.tobiasschmidt.quantus.QuantusProperties
import app.tobiasschmidt.quantus.session.SessionCalculator
import app.tobiasschmidt.quantus.session.SessionStore
import org.springframework.http.HttpStatus
import org.springframework.web.bind.annotation.DeleteMapping
import org.springframework.web.bind.annotation.GetMapping
import org.springframework.web.bind.annotation.PathVariable
import org.springframework.web.bind.annotation.PostMapping
import org.springframework.web.bind.annotation.RequestBody
import org.springframework.web.bind.annotation.RequestMapping
import org.springframework.web.bind.annotation.RequestParam
import org.springframework.web.bind.annotation.ResponseStatus
import org.springframework.web.bind.annotation.RestController

/**
 * The REST surface the desktop app talks to (spec §5). LaTeX goes in, steps and LaTeX come
 * back out; the session in between keeps the variables.
 */
@RestController
@RequestMapping("/api")
class CalculatorController(
    private val sessions: SessionStore,
    private val calculator: SessionCalculator,
    private val properties: QuantusProperties,
) {

    /**
     * What the Electron main process polls before it shows the window — it is also the answer
     * to "is a backend already running on this port?".
     */
    @GetMapping("/health")
    fun health() = mapOf(
        "status" to "ok",
        "application" to "quantus",
        "precision" to properties.precision,
    )

    @PostMapping("/evaluate")
    fun evaluate(@RequestBody request: CalculationRequest): CalculationResponse {
        val session = sessions.session(request.sessionId)
        val calculation = calculator.evaluate(session, request.latex, request.precision)
        val defined = calculation.formula.label?.key?.let { key -> session.variables().find { it.key == key } }
        return CalculationResponse.of(calculation, defined)
    }

    @GetMapping("/variables")
    fun variables(@RequestParam(required = false) sessionId: String?): List<VariableResponse> =
        sessions.session(sessionId).variables().map { VariableResponse.of(it) }

    @PostMapping("/variables")
    fun define(@RequestBody request: CalculationRequest): VariableResponse {
        val session = sessions.session(request.sessionId)
        return VariableResponse.of(calculator.define(session, request.latex, request.precision))
    }

    @DeleteMapping("/variables/{key}")
    @ResponseStatus(HttpStatus.NO_CONTENT)
    fun forget(@PathVariable key: String, @RequestParam(required = false) sessionId: String?) {
        sessions.session(sessionId).remove(key)
    }

    @DeleteMapping("/variables")
    @ResponseStatus(HttpStatus.NO_CONTENT)
    fun forgetAll(@RequestParam(required = false) sessionId: String?) {
        sessions.session(sessionId).clear()
    }
}
