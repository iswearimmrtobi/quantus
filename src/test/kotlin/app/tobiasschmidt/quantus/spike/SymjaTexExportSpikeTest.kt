package app.tobiasschmidt.quantus.spike

import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test
import org.matheclipse.core.eval.ExprEvaluator
import org.matheclipse.core.form.tex.TeXFormFactory
import org.matheclipse.core.interfaces.IExpr
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

/**
 * Phase 0 / Spike 3: Taugt Symjas eingebauter TeX-Export als Serializer-Basis?
 *
 * Kurzfassung: als Basis für den mathematischen Teil ja, für die Schritt-Darstellung
 * nach F6/F7 nicht — Einheiten-Markup, deutsches Dezimalkomma und die Anordnung
 * Zahl·Einheit fehlen. Details in docs/phase0-findings.md.
 */
class SymjaTexExportSpikeTest {

    private val evaluator = ExprEvaluator()

    private fun report(label: String, value: Any?) = println("  [tex] $label = $value")

    private fun tex(expr: IExpr): String {
        val sb = StringBuilder()
        TeXFormFactory().convert(sb, expr)
        return sb.toString().trim()
    }

    private fun texOfEvaluated(input: String): String = tex(evaluator.eval(input)).also {
        report(input, it)
    }

    @Test
    @DisplayName("Grundformen: Bruch, Wurzel, Potenz, Produkt")
    fun basicForms() {
        assertEquals("\\frac{\\sqrt{x}}{3}", texOfEvaluated("Sqrt(x)/3"))
        assertEquals("{x}^{2}", texOfEvaluated("x^2"))
        assertEquals("\\frac{1}{2} \\cdot m \\cdot {v}^{2}", texOfEvaluated("1/2*m*v^2"))
        assertEquals("\\frac{a}{b \\cdot c}", texOfEvaluated("a/b/c"))

        // Erfreulich: / wird von Haus aus zu \frac, und a/b/c landet flach in einem
        // Nenner — genau die Auto-Bruch-Regel aus F2.
    }

    @Test
    @DisplayName("Analysis-Ergebnisse rendern sauber")
    fun calculusForms() {
        assertEquals(
            "{x}^{2} \\cdot \\cos (x)+2 \\cdot x \\cdot \\sin (x)",
            texOfEvaluated("D(x^2*Sin(x), x)"),
        )
        assertEquals("- x \\cdot \\cos (x) + \\sin (x)", texOfEvaluated("Integrate(x*Sin(x), x)"))
        assertEquals("\\int  x \\cdot \\sin (x)\\,\\mathrm{d}x", tex(evaluator.parse("Integrate(x*Sin(x), x)")))
        assertEquals("\\frac{\\partial f(x)}{\\partial x}", tex(evaluator.parse("D(f(x), x)")))
    }

    @Test
    @DisplayName("Komplexe Zahlen: nur ausgewertete Ausdrücke rendern korrekt")
    fun complexOnlyRendersWhenEvaluated() {
        assertEquals("11 - 2\\,i", texOfEvaluated("(3 + 4*I) * (1 - 2*I)"))
        assertEquals("3 + 4\\,i", texOfEvaluated("3 + 4*I"))

        // Gegenprobe: derselbe Ausdruck ungeparst/unausgewertet kommt kaputt heraus.
        val unevaluated = tex(evaluator.parse("11 - I*2"))
        report("parse(\"11 - I*2\") unausgewertet", unevaluated)
        assertEquals("11\\left(  - i \\right)  \\cdot 2", unevaluated)
    }

    @Test
    @DisplayName("Variablen mit Index: Subscript() rendern, nackte Namen nicht")
    fun subscripts() {
        assertEquals("{r}_{1}", texOfEvaluated("Subscript(r, 1)"))
        assertEquals("{\\omega}_{0}", texOfEvaluated("Subscript(\\[Omega], 0)"))

        // r1 als Symbolname bleibt r1 — die Variablen aus F5 müssen also als
        // Subscript(name, index) in den AST, nicht als zusammengesetzter Symbolname.
        assertEquals("r1", texOfEvaluated("r1"))
    }

    @Test
    @DisplayName("HoldForm trägt den Schritt 'eingesetzt, unausgewertet' (F6)")
    fun holdFormForSubstitutedStep() {
        val held = evaluator.eval("HoldForm(2*981/100)")
        report("HoldForm ausgewertet", held)
        report("HoldForm als TeX", tex(held))

        assertEquals("2*981/100", held.toString(), "HoldForm verhindert die Auswertung")
        assertEquals("2 \\cdot \\frac{981}{100}", tex(held), "der Wrapper taucht im TeX nicht auf")

        // Aber: 981/100 statt 9{,}81 — der Serializer muss Dezimalzahlen selbst
        // formatieren, sonst steht im Rechenweg ein Bruch statt eines Messwerts.
    }

    @Test
    @DisplayName("Lücke 1: keine Einheiten-Semantik im TeX-Output")
    fun unitsAreNotMarkedUp() {
        val output = texOfEvaluated("2*kg*10*m/s^2")
        report("erwartet wäre", "20\\,\\mathrm{\\frac{kg \\cdot m}{s^2}}")

        assertEquals("\\frac{20 \\cdot kg \\cdot m}{{s}^{2}}", output)
        assertFalse(output.contains("\\mathrm"), "Einheiten bleiben kursive Variablen")
        assertFalse(output.contains("\\,"), "kein schmales Leerzeichen zwischen Zahl und Einheit")
    }

    @Test
    @DisplayName("Lücke 2: Dezimaltrennzeichen ist der Punkt, nicht das deutsche Komma")
    fun decimalSeparatorIsAPoint() {
        val output = texOfEvaluated("2*9.81")
        assertEquals("19.62", output)
        assertFalse(output.contains("{,}"), "F6 verlangt 19{,}62")
    }

    @Test
    @DisplayName("Lücke 3: hohe Präzision rendert ohne Marker, aber unformatiert")
    fun highPrecisionRendering() {
        val output = texOfEvaluated("N(Pi, 50)")
        assertTrue(output.startsWith("3.14159265358979323846"))
        assertFalse(output.contains("`"), "der Präzisionsmarker aus toString() fehlt im TeX — gut")
        assertTrue(output.length >= 50)

        // Große/kleine Zahlen: Symja weicht auf Exponentialschreibweise aus — und
        // schreibt dort ein nacktes "*" statt \cdot. Auch das muss der Serializer richten.
        val scientific = texOfEvaluated("N(2^200, 20)")
        assertEquals("1.6069380442589902755*10^{60}", scientific)
        assertTrue(scientific.contains("*10^"), "kein \\cdot 10^{…}")
    }
}
