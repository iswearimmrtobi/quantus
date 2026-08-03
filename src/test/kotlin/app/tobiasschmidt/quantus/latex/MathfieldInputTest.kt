package app.tobiasschmidt.quantus.latex

import app.tobiasschmidt.quantus.eval.LatexCalculator
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test
import kotlin.test.assertEquals

/**
 * Der Parser gegen das, was der MathLive-Editor tatsächlich ausgibt.
 *
 * Die Eingaben hier sind nicht ausgedacht, sondern aus dem laufenden Eingabefeld abgelesen:
 * `2/3` wird dort zu `\frac23`, jede Einheit landet in `\operatorname{}`, Klammern kommen mit
 * `\left`/`\right`. Wer den Parser nur gegen handgeschriebenes LaTeX testet, übersieht genau
 * das — und dann scheitert die App an ihrer eigenen Tastatureingabe.
 */
class MathfieldInputTest {

    private fun roundTrip(input: String): String = LatexSerializer.formula(LatexParser(input).parse())

    private fun result(input: String): String = LatexCalculator().evaluate(input).result

    @Test
    @DisplayName("argumentlose Kurzform: \\frac23 ist zwei Drittel, nicht dreiundzwanzig")
    fun braceLessArguments() {
        assertEquals("\\frac{2}{3}", roundTrip("\\frac23"))
        assertEquals("\\frac{1}{2}", roundTrip("\\frac12"))
        assertEquals("0{,}5", result("\\frac12"))
        assertEquals("\\sqrt{2}", roundTrip("\\sqrt2"))
        // Die Wurzel greift nur über die 4 — alles danach steht daneben, so liest LaTeX das.
        assertEquals("2\\,\\mathrm{m}^{2}", result("\\sqrt4m^2"))
        assertEquals("2\\,\\mathrm{m}", result("\\sqrt{4m^2}"))
    }

    @Test
    @DisplayName("mehrstellige Zahlen werden nur getrennt, wenn keine Klammern da sind")
    fun splittingNumbers() {
        // \frac123 ist (1/2)·3 — die 1 ist der Zähler, die 2 der Nenner, die 3 steht daneben.
        assertEquals("\\frac{1}{2} \\cdot 3", roundTrip("\\frac123"))
        assertEquals("\\frac{12}{34}", roundTrip("\\frac{12}{34}"))
        // Eine Dezimalzahl bleibt heil, statt bei der ersten Ziffer zu zerfallen.
        assertEquals("\\frac{1{,}5}{2}", roundTrip("\\frac1{,}5{2}"))
    }

    @Test
    @DisplayName("\\operatorname{} ist die Einheiten-Schreibweise des Editors")
    fun operatorname() {
        assertEquals("2\\,\\mathrm{kg}", roundTrip("2\\operatorname{kg}"))
        // So verschachtelt es MathLive tatsächlich.
        assertEquals("2\\,\\mathrm{kg}", roundTrip("2\\operatorname{\\mathrm{kg}}"))
        assertEquals("20\\,\\mathrm{N}", result("2\\operatorname{kg}\\cdot10\\frac{\\operatorname{m}}{\\operatorname{s}^2}"))
    }

    @Test
    @DisplayName("was der Editor beim Tippen der Meilenstein-Formel erzeugt, rechnet durch")
    fun typedMilestone() {
        // Zeichen für Zeichen `F = 2kg \cdot 10 m/s^2` getippt ergibt im Feld genau das hier.
        val typed = "F=2\\operatorname{kg}\\cdot10\\frac{\\operatorname{m}}{\\operatorname{s}^2}"
        assertEquals("F = 20\\,\\mathrm{N}", LatexCalculator().evaluate(typed).latex)
    }

    @Test
    @DisplayName("\\left und \\right um Klammern stören nicht")
    fun delimiters() {
        assertEquals("(2 + 3) \\cdot 4", roundTrip("\\left(2+3\\right)\\cdot4"))
    }
}
