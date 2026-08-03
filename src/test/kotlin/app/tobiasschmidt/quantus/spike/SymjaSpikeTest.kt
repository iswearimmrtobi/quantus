package app.tobiasschmidt.quantus.spike

import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test
import org.matheclipse.core.eval.EvalEngine
import org.matheclipse.core.eval.ExprEvaluator
import org.matheclipse.core.expression.F
import java.math.BigDecimal
import kotlin.test.assertEquals
import kotlin.test.assertTrue

/**
 * Phase 0 / Spike 1: Symja-API kennenlernen.
 *
 * Beantwortet: Wie baut und wertet man Ausdrücke? Wie wird die Präzision gesetzt?
 * Wie kommt man an der Schnittstelle an einen exakten Dezimalwert?
 * Alle Erkenntnisse zusammengefasst in docs/phase0-findings.md.
 */
class SymjaSpikeTest {

    private fun report(label: String, value: Any?) = println("  [symja] $label = $value")

    @Test
    @DisplayName("Ausdruck programmatisch bauen und auswerten (F-Factory)")
    fun buildAndEvaluate() {
        // Reihenfolge ist wichtig: erst den Evaluator anlegen (er erzeugt eine eigene
        // EvalEngine samt Context), dann die Symbole bauen. Andersherum zeigt
        // symbolsMustBeBuiltAfterTheEvaluator() das Ergebnis.
        val evaluator = ExprEvaluator()
        evaluator.defineVariable("x", F.ZZ(3))

        val expr = F.Times(F.ZZ(2), F.symbol("x"))
        val result = evaluator.eval(expr)
        report("2*x mit x=3", result)
        assertEquals("6", result.toString())
    }

    @Test
    @DisplayName("Stolperfalle: Symbole erst nach dem ExprEvaluator erzeugen")
    fun symbolsMustBeBuiltAfterTheEvaluator() {
        val expr = F.Times(F.ZZ(2), F.symbol("x")) // vor dem Evaluator gebaut
        val evaluator = ExprEvaluator()
        evaluator.defineVariable("x", F.ZZ(3))

        report("getVariable(x)", evaluator.getVariable("x"))
        report("eval(vorab gebautes IExpr)", evaluator.eval(expr))
        report("eval(\"2*x\")", evaluator.eval("2*x"))

        // Das vorab gebaute Symbol zeigt auf einen anderen Context und wird NICHT ersetzt.
        assertEquals("2*x", evaluator.eval(expr).toString())
        assertEquals("6", evaluator.eval("2*x").toString())
    }

    @Test
    @DisplayName("Symja kennt keine Einheiten — freie Bezeichner bleiben Symbole")
    fun symjaHasNoUnitSemantics() {
        val evaluator = ExprEvaluator()
        val evaluated = evaluator.eval("2*kg*10*m/s^2")
        report("2*kg*10*m/s^2", evaluated)

        // Symja rechnet die Zahlen zusammen und sortiert die Symbole — ohne jede
        // Dimensionsprüfung. Genau dafür ist die Indriya-Seite zuständig.
        assertEquals("(20*kg*m)/s^2", evaluated.toString())
        val nonsense = evaluator.eval("1*m + 1*s")
        report("1*m + 1*s (dimensional unsinnig)", nonsense)
        assertEquals("m+s", nonsense.toString())
    }

    @Test
    @DisplayName("Exakte Arithmetik: Rationalzahlen bleiben rational")
    fun exactRationals() {
        val evaluator = ExprEvaluator()
        val result = evaluator.eval("1/3 + 1/6")
        report("1/3 + 1/6", result)
        assertEquals("1/2", result.toString())

        val big = evaluator.eval("100!")
        report("100! Stellen", big.toString().length)
        assertTrue(big.toString().length > 150)
    }

    @Test
    @DisplayName("Beliebige Präzision: N(expr, stellen) ist der verlässliche Weg")
    fun arbitraryPrecisionViaN() {
        val evaluator = ExprEvaluator()
        val pi50 = evaluator.eval(F.N(F.Pi, 50))
        report("N(Pi, 50)", pi50)

        // toString hängt den Präzisionsmarker `50 an — für den LaTeX-Output abschneiden.
        assertTrue(pi50.toString().contains("`50"))
        val digits = pi50.toString().substringBefore("`").filter { it.isDigit() }.length
        report("signifikante Stellen", digits)
        assertTrue(digits >= 50, "erwartet >= 50 signifikante Stellen, gezählt: $digits")
        assertTrue(pi50.toString().startsWith("3.14159265358979323846"))
    }

    @Test
    @DisplayName("setNumericPrecision allein genügt nicht — N() braucht die Stellenzahl explizit")
    fun enginePrecisionAloneIsNotEnough() {
        val evaluator = ExprEvaluator()
        evaluator.evalEngine.setNumericPrecision(50)
        evaluator.evalEngine.significantFigures = 50

        val implicit = evaluator.eval("N(Sqrt(2))")
        val explicit = evaluator.eval("N(Sqrt(2), 50)")
        report("N(Sqrt(2)) bei engine-precision=50", implicit)
        report("N(Sqrt(2), 50)", explicit)

        // Ohne zweites Argument bleibt es bei Maschinengenauigkeit.
        assertEquals("1.41421", implicit.toString())
        assertTrue(explicit.toString().startsWith("1.4142135623730950488016887242096980785696718753769"))
    }

    @Test
    @DisplayName("Dezimalliterale werden zu Doubles — Eingaben als Rationalzahl parsen (F3)")
    fun decimalLiteralsCollapseToDoublePrecision() {
        val evaluator = ExprEvaluator()
        val asDouble = evaluator.eval("N(Sqrt(2.0), 30)")
        val asRational = evaluator.eval("N(Sqrt(2), 30)")
        report("N(Sqrt(2.0), 30) — Literal 2.0", asDouble)
        report("N(Sqrt(2), 30)   — exakte 2", asRational)

        // 2.0 ist ein Maschinen-Double: die Präzision ist bei 16 Stellen gedeckelt,
        // N(..., 30) holt sie nicht zurück.
        assertTrue(asDouble.toString().contains("`16"))
        assertTrue(asRational.toString().contains("`30"))

        // Konsequenz für den LaTeX-Parser: "9,81" -> 981/100, nicht -> 9.81
        val exact = evaluator.eval("N(981/100 * 2, 30)")
        report("2 * 981/100 exakt", exact)
        assertEquals("19.62", BigDecimal(exact.evalReal()!!.apfloatValue().toString()).toPlainString())
    }

    @Test
    @DisplayName("Schnittstelle nach außen: IExpr -> Apfloat -> BigDecimal")
    fun toBigDecimalBoundary() {
        val evaluator = ExprEvaluator()
        val value = evaluator.eval("N(Pi, 50)")
        report("isNumber / isReal", "${value.isNumber} / ${value.isReal}")

        val apfloat = value.evalReal()!!.apfloatValue()
        report("apfloatValue", apfloat)
        report("apfloat.precision", apfloat.precision())

        val asBigDecimal = BigDecimal(apfloat.toString())
        report("als BigDecimal", asBigDecimal.toPlainString())
        assertEquals(50L, apfloat.precision())
        assertTrue(asBigDecimal.toPlainString().startsWith("3.14159265358979323846"))
    }

    @Test
    @DisplayName("Komplexe Zahlen (F4)")
    fun complexNumbers() {
        val evaluator = ExprEvaluator()
        val product = evaluator.eval("(3 + 4*I) * (1 - 2*I)")
        report("(3+4i)*(1-2i)", product)
        assertEquals("11-I*2", product.toString())

        report("Abs(3+4i)", evaluator.eval("Abs(3 + 4*I)"))
        report("Arg(3+4i)", evaluator.eval("N(Arg(3 + 4*I), 20)"))
        report("kartesisch -> polar", evaluator.eval("N({Abs(3+4*I), Arg(3+4*I)}, 20)"))
        report("polar -> kartesisch", evaluator.eval("ExpToTrig(5*E^(I*Pi/2))"))
        assertEquals("5", evaluator.eval("Abs(3 + 4*I)").toString())
    }

    @Test
    @DisplayName("Analysis: Ableitung und unbestimmtes Integral (F8)")
    fun calculus() {
        val evaluator = ExprEvaluator()
        val x = F.symbol("x")

        val derivative = evaluator.eval(F.D(F.Times(F.Power(x, F.C2), F.Sin(x)), x))
        report("d/dx x^2*sin(x)", derivative)
        assertEquals("x^2*Cos(x)+2*x*Sin(x)", derivative.toString())

        val integral = evaluator.eval(F.Integrate(F.Times(x, F.Sin(x)), x))
        report("∫ x*sin(x) dx", integral)
        assertEquals("-x*Cos(x)+Sin(x)", integral.toString())

        report("∂/∂y x^2*y^3", evaluator.eval("D(x^2*y^3, y)"))
        report("d²/dx² x^4", evaluator.eval("D(x^4, {x, 2})"))
        assertEquals("12*x^2", evaluator.eval("D(x^4, {x, 2})").toString())
    }

    @Test
    @DisplayName("EvalEngine ist thread-lokal — relevant für den Spring-Boot-Betrieb")
    fun engineIsThreadLocal() {
        val mainEngine = EvalEngine.get()
        var workerEngine: EvalEngine? = null
        val thread = Thread { workerEngine = EvalEngine.get() }
        thread.start()
        thread.join()

        report("engine main", System.identityHashCode(mainEngine))
        report("engine worker", System.identityHashCode(workerEngine))
        assertTrue(
            mainEngine !== workerEngine,
            "EvalEngine.get() liefert pro Thread eine eigene Instanz",
        )
    }

    @Test
    @DisplayName("Laufzeit typischer Auswertungen (N3: < 200 ms)")
    fun performance() {
        val evaluator = ExprEvaluator()
        evaluator.eval("N(Pi, 50)")

        val cases = listOf(
            "2 * 981/100",
            "N(Pi, 50)",
            "D(x^2*Sin(x), x)",
            "(3 + 4*I) * (1 - 2*I)",
        )
        for (case in cases) {
            val nanos = System.nanoTime()
            evaluator.eval(case)
            val millis = (System.nanoTime() - nanos) / 1_000_000.0
            report("Laufzeit '$case'", "$millis ms")
            assertTrue(millis < 200, "N3 verletzt für '$case': $millis ms")
        }

        // Integrate lädt beim ersten Aufruf ein großes Regelwerk nach — kalt weit über
        // 200 ms, warm darunter. Konsequenz: beim Backend-Start vorwärmen.
        var nanos = System.nanoTime()
        evaluator.eval("Integrate(x*Sin(x), x)")
        report("Integrate kalt", "${(System.nanoTime() - nanos) / 1_000_000.0} ms")

        nanos = System.nanoTime()
        evaluator.eval("Integrate(x^3*Cos(x), x)")
        val warm = (System.nanoTime() - nanos) / 1_000_000.0
        report("Integrate warm", "$warm ms")
    }
}
