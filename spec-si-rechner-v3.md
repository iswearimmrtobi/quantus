# Spec Sheet: SI-Einheiten-Rechner ("Quantus")

**Stand:** 2026-08-18 · **Status:** Konzeptphase (Tech-Stack-Revision)

Diese Version ersetzt [spec-si-rechner-v2.md](spec-si-rechner-v2.md) in Abschnitt 4/5
(Technologie-Entscheidungen, API). Die funktionalen Anforderungen (F1–F11) sind unverändert.
Die Roadmap ist ausgelagert nach [roadmap-si-rechner-v3.md](roadmap-si-rechner-v3.md).

---

## 1. Projektziel

Ein Taschenrechner-Programm für physikalische Berechnungen, das mit SI-Einheiten dimensional korrekt rechnet, symbolische Mathematik beherrscht (Ableitungen, unbestimmte Integrale, komplexe Zahlen) und Rechnungen so präsentiert, wie man sie im Physikunterricht aufschreibt: Formel → Einsetzen → Ergebnis, mit kopierbarem LaTeX-Output.

**Referenzbeispiel:**
Eingabe `1\,\mathrm{kg} \times 1\,\mathrm{m/s^2}` → Ergebnis `1\,\mathrm{N}` inkl. Rechenweg.

---

## 2. Funktionale Anforderungen

### F1 — Einheitenrechnung (Kern)
- Rechnen mit SI-Basiseinheiten und abgeleiteten Einheiten (N, J, W, Pa, V, Ω, …)
- Automatische Dimensionsprüfung: Addition inkompatibler Größen → verständlicher Fehler
- Automatische Vereinfachung auf benannte abgeleitete Einheiten (kg·m/s² → N)
- Einheitenkonvertierung (km/h ↔ m/s, Präfixe: k, m, µ, M, …)

### F2 — LaTeX-Eingabe
- Parsen von LaTeX-Ausdrücken: Zahlen, `\frac`, `\sqrt`, `^`, `\times`, `\cdot`, Klammern
- **Einheiten ohne Markup:** nackte Buchstabenfolgen nach Zahlen bzw. in `\frac{}{}` werden als Einheiten geparst — `F = 2kg \cdot 10\frac{m}{s^2}` ist valide Eingabe. `\mathrm{…}` / `\text{…}` und siunitx-Syntax (`\qty{12}{cm}`) bleiben zusätzlich unterstützt
- **Eindeutigkeitsregel:** Variablen MÜSSEN immer einen Index tragen (`r_1`, `v_1`, `\omega_0`) — ein Bezeichner ohne Subskript ist damit immer eine Einheit oder eine Konstante, nie eine Variable. Konstanten (π, e, i sowie später vordefinierte Naturkonstanten) haben reservierte Namen und kollidieren nicht mit Einheiten-Symbolen
- **Auto-Bruch-Umbau:** jedes `/` wird beim Rendern automatisch zu `\frac{}{}`, unabhängig von Einheiten — Operandengrenzen nach Standard-Präzedenz (`\cdot`/`/` gleichrangig, linksassoziativ), Ketten wie `a/b/c` werden zu einem `\frac{a}{b\cdot c}` zusammengefasst statt verschachtelt, explizite Klammern gewinnen immer

### F3 — Hohe Genauigkeit
- Beliebige Präzision (arbitrary precision) statt fester 64/128-Bit-Floats
- Konfigurierbare Stellenanzahl (Default: z. B. 50 signifikante Stellen)
- Exakte Arithmetik wo möglich (Rationalzahlen, symbolische Konstanten wie π, e)

### F4 — Komplexe Zahlen
- Rechnen mit komplexen Zahlen in kartesischer und Polarform
- Ausgabe wahlweise als a+bi oder r·e^(iφ)

### F5 — Variablendefinition
- Benutzer definiert Variablen mit Wert und Einheit, z. B. `r_1 = 0{,}5\,\mathrm{m}` oder kurz `r_1 = 0{,}5m`
- **Variablennamen müssen einen Index tragen** (siehe F2) — Definitionen ohne Subskript werden mit verständlicher Fehlermeldung abgelehnt
- Variablen bleiben in der Session gültig und werden in Folgeausdrücken aufgelöst
- Variablen erscheinen im LaTeX-Output symbolisch UND mit eingesetzten Werten (siehe F6)

### F6 — Rechnung in Schritten
Darstellung wie im Physikunterricht, drei Zustände pro Rechnung:
1. **Symbolisch:** `F = m \cdot a`
2. **Eingesetzt (unausgewertet):** `F = 2\,\mathrm{kg} \cdot 9{,}81\,\mathrm{m/s^2}`
3. **Ergebnis:** `F = 19{,}62\,\mathrm{N}`

### F7 — LaTeX-Output
- Jeder Rechenschritt und das Endergebnis als kopierbarer LaTeX-Code
- Ein-Klick-Copy pro Schritt sowie für die komplette Rechnung (z. B. als `align*`-Block)

### F8 — Analysis
- Symbolische Ableitungen (auch partielle, höhere Ordnung)
- Unbestimmte Integrale (symbolisch)
- Ergebnis ebenfalls als LaTeX

### F9 — History
- Persistente Historie aller Rechnungen (inkl. definierter Variablen)
- Frühere Rechnungen anklicken → erneut laden/bearbeiten
- Suche/Filter in der Historie (nice-to-have)

### F10 — Desktop-taugliche UI
- Optimiert für Tastatur-/Numpad-Eingabe: Ziffernblock, Operatoren, Enter = auswerten
- Mathematischer Eingabe-Editor mit Live-Rendering (LaTeX sichtbar während der Eingabe)
- Tastenkürzel für häufige Aktionen (neue Rechnung, Copy LaTeX, Variable definieren)
- Sichtbares Variablen-Panel (aktuelle Definitionen der Session)

### F11 — Größennamen-Erkennung (informativ)
- Zweite, kleinere Lookup-Tabelle: Dimension → gebräuchlicher physikalischer Name (L/T → „Geschwindigkeit", L/T² → „Beschleunigung", M/L³ → „Dichte", …)
- Rein informatives Label neben dem Ergebnis, ändert nichts an der Einheit selbst
- Ergänzt F1 (SI-Einheiten-Vereinfachung), greift dort wo keine benannte SI-Einheit existiert (z. B. m/s)

---

## 3. Nicht-funktionale Anforderungen

| # | Anforderung |
|---|---|
| N1 | Kernlogik (CAS, Einheiten, LaTeX-I/O) in **Kotlin**, läuft im selben Prozess wie die UI — kein separates Backend |
| N2 | UI als **natives Desktop-Programm** (JavaFX/Kotlin); MathLive als Eingabe-Widget, eingebettet in eine JavaFX-`WebView` (WebKit, in der JavaFX-SDK gebündelt) |
| N2a | Gesamte App läuft als **ein einzelner JVM-Prozess** — keine Kindprozess-Verwaltung, kein Port-Handshake, kein REST/JSON zwischen UI und Kernlogik |
| N3 | Antwortzeit für typische Auswertungen < 200 ms (ohne HTTP-Roundtrip realistischer als zuvor) |
| N4 | *(Backlog, nicht Teil von v1)* Optionaler Self-hosted-Betrieb über eine dünne HTTP-Fassade (Ktor/Javalin) vor denselben Kernmodulen, falls später ein Server-Szenario gebraucht wird |
| N5 | Saubere Trennung: CAS-Logik, Einheiten-Logik, LaTeX-I/O als getrennte Module — unverändert gegenüber v2 |

---

## 4. Technologie-Entscheidungen

| Baustein | Wahl | Begründung |
|---|---|---|
| Sprache/Runtime | Kotlin, ein JVM-Prozess | Kein Backend/Frontend-Split mehr nötig — Kotlin trägt sowohl Kernlogik als auch UI |
| CAS | **Symja** | Reines Java-CAS: komplexe Zahlen, Ableitungen, Integrale, arbitrary precision, Pattern Matching — nativ auf der JVM (unverändert) |
| Einheiten | **Indriya** (JSR-385 RI) | Dimensionsprüfung, Unit-Arithmetik, SI-System vollständig (unverändert) |
| Präzision | Symja intern (Apfloat) + BigDecimal an den Schnittstellen | Beliebige Genauigkeit (unverändert) |
| LaTeX-Parser | **Eigenbau** (recursive descent) | Kein reifes JVM-Pendant zu sympy vorhanden; benötigter Subset ist überschaubar (unverändert) |
| LaTeX-Serializer | **Eigenbau** (AST → LaTeX) | Volle Kontrolle über Schritt-Darstellung (unverändert) |
| UI | **JavaFX** (Kotlin) | Natives Desktop-Toolkit, läuft im selben Prozess wie die Kernlogik — direkte Funktionsaufrufe statt REST/JSON |
| Mathe-Eingabe-Editor | **MathLive** in einer JavaFX-`WebView` | WebView ist WebKit, in der JavaFX-SDK gebündelt — kein Electron/Chromium, kein Node/npm-Toolchain. Nur der Editor läuft im Web-Kontext; Ergebnisanzeige, Schritte, Variablen-Panel und History sind natives JavaFX/CSS |
| Persistenz | **Exposed** (oder plain JDBC) + **SQLite** (Datei im User-Profil) | Gleiche Datei wie zuvor, aber ohne Spring Data JPA/Hibernate-Overhead für eine Handvoll Tabellen |
| Packaging | **jpackage** (über Gradle-Plugin, z. B. badass-jlink/runtime) | Ein JDK-natives Werkzeug statt zweier getrennter Pipelines (jlink + electron-builder); ein Installer pro Plattform (msi/deb/AppImage/dmg) inkl. gebündelter Minimal-JRE |
| Self-hosted (Backlog) | Dünne Ktor/Javalin-Fassade über denselben `cas`/`units`/`quantity`/`latex`/`eval`-Modulen | Nicht Teil von v1; die Module sind bereits UI-unabhängig, falls später doch ein Server-Betrieb gebraucht wird |

### Architektur-Kernentscheidung: Symja ⟷ Indriya

Unverändert gegenüber v2 — es existiert **keine fertige Brücke** zwischen beiden Libraries. Daher eigene Vermittlungsschicht:

```
PhysicalQuantity = (Symja IExpr: Zahlwert)  +  (Indriya Unit: Dimension)
```

- **Symja** rechnet alles Numerische/Symbolische (Wert, komplexe Zahlen, Analysis)
- **Indriya** führt parallel die Dimensions-Buchhaltung (Unit-Arithmetik bei ×/÷, Kompatibilitätsprüfung bei +/−)
- Die Vermittlungsschicht mappt jede Operation auf beide Seiten

Neu gegenüber v2 ist nur, dass diese Kernlogik nicht mehr hinter einer REST-Schnittstelle sitzt,
sondern direkt aus der UI-Schicht heraus aufgerufen wird:

```
┌───────────────────────────────────────────────┐
│              JavaFX-App (Kotlin)               │
│  ┌───────────────────────────────────────┐    │
│  │  WebView: MathLive-Editor              │    │
│  │  (nur der Eingabe-Editor läuft hier)   │    │
│  └──────────────────┬──────────────────────┘    │
│                     │ LaTeX-String (JS↔Kotlin-Bridge)
│  ┌──────────────────▼──────────────────────┐    │
│  │  Eval-Orchestrierung (Schritt-Erzeugung) │    │
│  │  LaTeX-Parser → PhysicalQuantity → Serial.│   │
│  │       (Symja ⟷ Indriya)                  │    │
│  └──────────────────┬──────────────────────┘    │
│  ┌──────────────────▼──────────────────────┐    │
│  │  History (Exposed/JDBC + SQLite)         │    │
│  └───────────────────────────────────────┘    │
│  Ergebnis, Schritte, Variablen-Panel: natives   │
│  JavaFX/CSS, kein Roundtrip über HTTP           │
└───────────────────────────────────────────────┘
```

---

## 5. Interne Schnittstelle

Kein REST-API mehr im Kern der App — Eval-Orchestrierung, Variablenverwaltung und History
werden als direkte Kotlin-Funktionsaufrufe aus der JavaFX-Schicht angesprochen. Die Form bleibt
nah an der bisherigen API-Skizze, nur ohne HTTP/JSON:

```kotlin
fun evaluate(latex: String, sessionId: SessionId, precision: Int = 50): EvaluationResult
// EvaluationResult(steps: List<Step>, value: String, unit: String, error: String?)
// Step(kind: SYMBOLIC | SUBSTITUTED | RESULT, latex: String)

fun defineVariable(sessionId: SessionId, name: String, index: String, value: PhysicalQuantity)
fun getVariables(sessionId: SessionId): List<VariableDefinition>
fun getHistory(query: String? = null): List<HistoryEntry>
fun derive(expr: String, wrt: String): EvaluationResult
fun integrate(expr: String, wrt: String): EvaluationResult
```

**Backlog:** Sollte der optionale Self-hosted-Betrieb (N4) später gebraucht werden, bekommt genau
diese Funktionssignatur eine dünne Ktor/Javalin-Fassade vorgesetzt (REST/JSON nach außen, gleiche
Aufrufe nach innen) — die Kernmodule ändern sich dafür nicht.

---

## 6. Risiken

| Risiko | Auswirkung | Gegenmaßnahme |
|---|---|---|
| LaTeX-Parser wird unterschätzt | Verzögerung | Subset klein halten, ANTLR als Fallback |
| Mehrdeutigkeiten bei markuploser Einheiten-Erkennung (Präfix-Zerlegung: mN, min, …) | Falsch geparste Eingaben | Feste Regeln: Longest-Match gegen Einheiten-Tabelle vor Präfix-Zerlegung; implizite Einheitenprodukte verboten (immer `\cdot`); Testsuite mit allen bekannten Kollisionsfällen |
| Indriya erkennt abgeleitete Einheiten nicht automatisch als N/J/… | F1 unvollständig | Eigene Dimension→Name-Lookup-Tabelle (überschaubar: ~20 SI-Einheiten) |
| Symja-TeX-Export unbrauchbar für Schritt-Darstellung | Mehraufwand Serializer | Eigenbau ohnehin eingeplant, Export nur als Abkürzung |
| Einheiten + Analysis kombiniert ist konzeptionell schwierig | F8 eingeschränkt | Analysis zunächst einheitenlos ausliefern (Phase 4) |
| JS↔Kotlin-Bridge zur WebView (MathLive-Wert auslesen/setzen, Cursor-Events) ist fummelig | Verzögerung Phase 5, Editor-Bugs | Kleine, klar geschnittene Bridge-API (nur LaTeX-String + Cursor-Events), früh mit einem Spike absichern statt erst spät in Phase 5 |
| jpackage-Installer variiert stark je Plattform (Signierung, AppImage-Eigenheiten) | Installer-Probleme auf einzelnen OS | Früh auf allen Zielplattformen testen, Gradle-Plugin statt Handarbeit |
