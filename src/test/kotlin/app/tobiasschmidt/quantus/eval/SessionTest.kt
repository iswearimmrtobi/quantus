package app.tobiasschmidt.quantus.eval

import app.tobiasschmidt.quantus.cas.CasEngine
import app.tobiasschmidt.quantus.latex.VariableNode
import app.tobiasschmidt.quantus.quantity.PhysicalQuantity
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test
import tech.units.indriya.unit.Units
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertNull
import kotlin.test.assertTrue

/** Phase 3: the session's variable store and how `ExpressionEvaluator` resolves against it. */
class SessionTest {

    @Test
    @DisplayName("a fresh session has no variables and resolves nothing")
    fun emptySession() {
        val session = Session(SessionId.random())
        assertTrue(session.variables().isEmpty())
        assertNull(session.resolve(VariableKey("r", "1")))
    }

    @Test
    @DisplayName("a defined variable round-trips through resolve() and variables()")
    fun defineAndResolve() {
        CasEngine()
        val session = Session(SessionId.random())
        val halfMetre = PhysicalQuantity.of(1, Units.METRE) / PhysicalQuantity.dimensionless(2)
        session.define(VariableKey("r", "1"), halfMetre)

        assertEquals(halfMetre, session.resolve(VariableKey("r", "1")))
        assertEquals(listOf(VariableDefinition(VariableKey("r", "1"), halfMetre)), session.variables())
    }

    @Test
    @DisplayName("SessionStore hands back the same session for the same id, a fresh one for a new id")
    fun store() {
        val store = SessionStore()
        val id = SessionId.random()
        val session = store.session(id)
        session.define(VariableKey("x", "1"), PhysicalQuantity.dimensionless(1))

        assertEquals(session, store.session(id))
        assertTrue(store.session(SessionId.random()).variables().isEmpty())
    }

    @Test
    @DisplayName("ExpressionEvaluator resolves a variable through the given lookup")
    fun expressionEvaluatorResolvesVariables() {
        CasEngine()
        val session = Session(SessionId.random())
        session.define(VariableKey("r", "1"), PhysicalQuantity.of(2, Units.METRE))

        val evaluator = ExpressionEvaluator(session::resolve)
        val node = VariableNode("r", "1", 0)
        assertEquals(PhysicalQuantity.of(2, Units.METRE), evaluator.evaluate(node))
    }

    @Test
    @DisplayName("without a resolver (or an unresolved key) the variable is rejected as before")
    fun defaultResolverStillRejects() {
        val node = VariableNode("v", "1", 0)
        val error = assertFailsWith<UnresolvedVariableException> { ExpressionEvaluator().evaluate(node) }
        assertTrue(error.message!!.contains("v_1"), error.message!!)
    }
}
