package app.tobiasschmidt.quantus.web

import org.hamcrest.Matchers.containsString
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test
import org.springframework.beans.factory.annotation.Autowired
import org.springframework.boot.test.context.SpringBootTest
import org.springframework.http.MediaType
import org.springframework.test.web.servlet.ResultActions
import org.springframework.test.web.servlet.request.MockMvcRequestBuilders.delete
import org.springframework.test.web.servlet.request.MockMvcRequestBuilders.get
import org.springframework.test.web.servlet.request.MockMvcRequestBuilders.post
import org.springframework.test.web.servlet.result.MockMvcResultMatchers.content
import org.springframework.test.web.servlet.result.MockMvcResultMatchers.jsonPath
import org.springframework.test.web.servlet.result.MockMvcResultMatchers.status
import org.springframework.test.web.servlet.setup.MockMvcBuilders
import org.springframework.web.context.WebApplicationContext

/**
 * The REST surface end to end — the contract the Electron app relies on (spec §5).
 *
 * Every test uses a session id of its own; sessions live in memory and would otherwise leak
 * their variables into each other.
 */
@SpringBootTest
class CalculatorControllerTest(@Autowired context: WebApplicationContext) {

    private val mvc = MockMvcBuilders.webAppContextSetup(context).build()

    private fun send(latex: String, session: String, path: String = "/api/evaluate"): ResultActions =
        mvc.perform(
            post(path)
                .contentType(MediaType.APPLICATION_JSON)
                .content("""{"latex": "${latex.replace("\\", "\\\\")}", "sessionId": "$session"}"""),
        )

    @Test
    @DisplayName("health tells the Electron main process that the backend is up")
    fun health() {
        mvc.perform(get("/api/health"))
            .andExpect(status().isOk)
            .andExpect(jsonPath("$.status").value("ok"))
            .andExpect(jsonPath("$.precision").value(50))
    }

    @Test
    @DisplayName("evaluate returns steps, result, unit and dimension")
    fun evaluate() {
        send("F = 2kg \\cdot 10\\frac{m}{s^2}", "evaluate")
            .andExpect(status().isOk)
            .andExpect(jsonPath("$.latex").value("F = 20\\,\\mathrm{N}"))
            .andExpect(jsonPath("$.result").value("20\\,\\mathrm{N}"))
            .andExpect(jsonPath("$.value").value("20"))
            .andExpect(jsonPath("$.unit").value("N"))
            .andExpect(jsonPath("$.quantityName").value("force"))
            .andExpect(jsonPath("$.dimension").isString)
            .andExpect(jsonPath("$.steps.length()").value(2))
            .andExpect(jsonPath("$.steps[0].kind").value("symbolic"))
            .andExpect(jsonPath("$.steps[1].kind").value("result"))
            .andExpect(jsonPath("$.alignBlock").value(containsString("\\begin{align*}")))
            .andExpect(jsonPath("$.variable").doesNotExist())
    }

    @Test
    @DisplayName("a definition comes back as a variable and resolves in the next call")
    fun variables() {
        send("m_1 = 2kg", "vars")
            .andExpect(status().isOk)
            .andExpect(jsonPath("$.variable.key").value("m_1"))
            .andExpect(jsonPath("$.variable.symbol").value("m_{1}"))
            .andExpect(jsonPath("$.variable.latex").value("2\\,\\mathrm{kg}"))

        send("a_1 = 10\\frac{m}{s^2}", "vars").andExpect(status().isOk)

        send("F = m_1 \\cdot a_1", "vars")
            .andExpect(status().isOk)
            .andExpect(jsonPath("$.latex").value("F = 20\\,\\mathrm{N}"))
            .andExpect(jsonPath("$.steps.length()").value(3))
            .andExpect(jsonPath("$.steps[1].kind").value("substituted"))
            .andExpect(
                jsonPath("$.steps[1].latex")
                    .value("F = 2\\,\\mathrm{kg} \\cdot 10\\,\\frac{\\mathrm{m}}{\\mathrm{s}^{2}}"),
            )

        mvc.perform(get("/api/variables").param("sessionId", "vars"))
            .andExpect(status().isOk)
            .andExpect(jsonPath("$.length()").value(2))
            .andExpect(jsonPath("$[0].key").value("m_1"))
            .andExpect(jsonPath("$[1].key").value("a_1"))
    }

    @Test
    @DisplayName("the variables endpoint defines and forgets")
    fun defineAndForget() {
        send("r_1 = 0{,}5m", "define", path = "/api/variables")
            .andExpect(status().isOk)
            .andExpect(jsonPath("$.key").value("r_1"))
            .andExpect(jsonPath("$.value").value("0.5"))

        mvc.perform(delete("/api/variables/r_1").param("sessionId", "define"))
            .andExpect(status().isNoContent)

        mvc.perform(get("/api/variables").param("sessionId", "define"))
            .andExpect(content().json("[]"))
    }

    @Test
    @DisplayName("an input that defines nothing is no variable definition")
    fun defineRejectsExpression() {
        send("2kg \\cdot 3", "define-error", path = "/api/variables")
            .andExpect(status().isBadRequest)
            .andExpect(jsonPath("$.error").value(containsString("does not define a variable")))
    }

    @Test
    @DisplayName("sessions are separate: a variable of one is unknown in the other")
    fun sessionsAreSeparate() {
        send("r_1 = 1m", "one").andExpect(status().isOk)

        send("2 \\cdot r_1", "two")
            .andExpect(status().isBadRequest)
            .andExpect(jsonPath("$.error").value(containsString("r_1")))
    }

    @Test
    @DisplayName("a parse error answers 400 with a readable message and the position")
    fun parseError() {
        send("2 \\cdot", "errors")
            .andExpect(status().isBadRequest)
            .andExpect(jsonPath("$.error").isString)
            .andExpect(jsonPath("$.position").isNumber)
    }

    @Test
    @DisplayName("a dimension error answers 400 and explains itself")
    fun dimensionError() {
        send("1m + 1s", "errors")
            .andExpect(status().isBadRequest)
            .andExpect(jsonPath("$.error").value("Incompatible units: m (length) and s (time) cannot be added."))
    }
}
