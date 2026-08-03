# Phase 0 — Erkenntnisse aus den Spikes

**Stand:** 2026-08-03 · **Status:** abgeschlossen

Grundlage sind die ausführbaren Spikes unter `src/test/kotlin/app/tobiasschmidt/quantus/spike/`.
Jede Aussage hier ist dort als Test festgenagelt — ändert eine Library-Version das
Verhalten, schlägt der zugehörige Test fehl.

```
./gradlew test
```

## Aufgesetzter Stack

| Baustein | Version | Bemerkung |
|---|---|---|
| Kotlin | 2.4.10 | JVM-Toolchain 21 |
| Spring Boot | 4.1.0 | `spring-boot-starter-web`, Port 8437 |
| Symja (`org.matheclipse:matheclipse-core`) | 3.2.0 | zieht Hipparchus + Apfloat mit |
| Indriya (`tech.units:indriya`) | 2.2.4 | JSR-385 RI, dazu `javax.measure:unit-api:2.2` |
| Tests | JUnit 5 via `spring-boot-starter-test` + `kotlin-test` | |

**Meilenstein erreicht:** Beide Libraries laufen im selben Projekt, es gibt keine
Classpath-Kollision, der Spring-Context startet mit beiden (`QuantusApplicationTests`).

---

## Spike 1 — Symja

### Was gut funktioniert

- **Ausdrucksbau** über die Factory `F` (`F.Times`, `F.Power`, `F.D`, `F.Integrate`, …),
  gemeinsamer AST-Typ ist `IExpr`. Auswertung über `ExprEvaluator.eval(IExpr | String)`.
- **Exakte Arithmetik:** `1/3 + 1/6` → `1/2`, `100!` exakt mit 158 Stellen.
- **Beliebige Präzision:** `N(Pi, 50)` liefert 50 signifikante Stellen.
- **Analysis (F8):** `D(x^2*Sin(x), x)` → `x^2*Cos(x)+2*x*Sin(x)`, `Integrate(x*Sin(x), x)` →
  `-x*Cos(x)+Sin(x)`, partielle und höhere Ableitungen (`D(x^4, {x,2})` → `12*x^2`) alle out of the box.
- **Komplexe Zahlen (F4):** `(3+4I)*(1-2I)` → `11-I*2`, `Abs`/`Arg`/`ExpToTrig` für die Polarform vorhanden.
- **Schnittstelle nach außen:** `IExpr.evalReal()!!.apfloatValue()` → `Apfloat` mit gemeldeter
  Präzision; `BigDecimal(apfloat.toString())` ist verlustfrei.

### Fallstricke, die die Architektur beeinflussen

1. **`setNumericPrecision()` allein genügt nicht.** Mit engine-weiter Präzision 50 liefert
   `N(Sqrt(2))` trotzdem `1.41421`. Die Stellenzahl muss als zweites Argument mitgegeben werden:
   `N(Sqrt(2), 50)`. → Die Eval-Orchestrierung baut `N(expr, precision)` immer explizit.
2. **Dezimalliterale werden Maschinen-Doubles.** `N(Sqrt(2.0), 30)` bleibt bei 16 Stellen
   (`1.414213562373095\`16`), `N(Sqrt(2), 30)` liefert 30. → **Der LaTeX-Parser muss Dezimalzahlen
   als Rationalzahl in den AST legen** (`9,81` → `981/100`), niemals als `9.81`. Sonst ist F3
   schon an der Eingabe verloren.
3. **Symbole erst nach dem `ExprEvaluator` erzeugen.** Ein vor der Evaluator-Konstruktion
   gebautes `F.symbol("x")` zeigt auf einen anderen Context und wird von `defineVariable("x", …)`
   nicht ersetzt — `eval` liefert dann stumm `2*x` statt `6`. Der String-Weg (`eval("2*x")`)
   ist davon nicht betroffen.
4. **`toString()` hängt einen Präzisionsmarker an:** `3.14159…751\`50`. Für den LaTeX-Output
   abschneiden — der TeX-Export selbst macht das bereits richtig.
5. **`EvalEngine` ist thread-lokal.** `EvalEngine.get()` liefert pro Thread eine eigene Instanz.
   Im Spring-Boot-Betrieb heißt das: pro Request eine eigene Engine bauen und konfigurieren,
   niemals eine Engine als Singleton-Bean über Requests teilen.
6. **Symja hat keinerlei Einheiten-Semantik.** `2*kg*10*m/s^2` wird klaglos zu `(20*kg*m)/s^2`,
   und `1*m + 1*s` zu `m+s`. Bestätigt die Kernentscheidung: die Dimensionsbuchhaltung muss
   komplett auf der Indriya-Seite laufen.

### Laufzeit (N3: < 200 ms)

| Ausdruck | Laufzeit |
|---|---|
| `2 * 981/100` | 0,3 ms |
| `N(Pi, 50)` | 0,5 ms |
| `D(x^2*Sin(x), x)` | 13 ms |
| `(3 + 4*I) * (1 - 2*I)` | 0,7 ms |
| `Integrate(x*Sin(x), x)` — kalt | **1033 ms** |
| `Integrate(x^3*Cos(x), x)` — warm | **594 ms** |

**Risiko für N3:** Arithmetik, Präzision und Ableitungen liegen mit großem Abstand unter
200 ms. Unbestimmte Integrale nicht — auch warm nicht. Gegenmaßnahmen für Phase 4:
Regelwerk beim Backend-Start vorwärmen (ein Dummy-`Integrate` im `ApplicationReadyEvent`),
Integration asynchron mit Timeout (`evaluateWithTimeout`) und N3 explizit auf
Nicht-Integral-Auswertungen beschränken.

---

## Spike 2 — Indriya

### Die Leitfrage: wird kg × m/s² automatisch zu N?

**Nein.** `Units.KILOGRAM.multiply(Units.METRE_PER_SQUARE_SECOND)` ergibt `kg·m/s²`, und
`product == Units.NEWTON` ist `false`.

Aber alles, was für die Benennung gebraucht wird, ist vorhanden:

- `product.dimension == Units.NEWTON.dimension` → `true` (die `toString`-Reihenfolge weicht ab —
  `[M]·[L]/[T]²` vs. `[L]·[M]/[T]²` — `equals`/`hashCode` stimmen trotzdem, `Dimension` taugt als Map-Key).
- `product.isCompatible(Units.NEWTON)` → `true`.
- `quantity.to(Units.NEWTON)` → **`19.62 N`**. Damit ist das Referenzbeispiel des Specs
  auf der Einheiten-Seite gelöst.

**Konsequenz für F1/Phase 1:** Vereinfachung = Lookup `Dimension → benannte Einheit`, dann `to()`.
`Units.getInstance().units` (42 Einheiten) ist ein brauchbarer Seed für diese Tabelle, reicht
aber nicht allein: Dimensionen sind **nicht eindeutig** — die Geschwindigkeits-Dimension liefert
`[m/s, km/h]`. Es braucht also eine eigene, priorisierte Tabelle mit der kohärenten SI-Einheit
als bevorzugtem Treffer. Größe wie im Spec geschätzt: ~20 Einträge.

### Weitere Ergebnisse

- **Dimensionsprüfung (+/−):** Der Generics-Typ `Unit<Q : Quantity<Q>>` fängt inkompatible
  Additionen schon zur Compile-Zeit. Zur Laufzeit (ungetypt) fliegt eine
  `javax.measure.UnconvertibleException: … s is not compatible with m`. Die Meldung ist zu
  technisch für F1 → `PhysicalQuantity` prüft in Phase 1 selbst per `isCompatible()` und
  formuliert eigene Fehlertexte.
- **Generics sind in der Praxis im Weg.** Zwischen zwei `Unit<*>` lässt sich nichts umrechnen.
  `PhysicalQuantity` wird intern ungetypt (`Unit<*>` + unchecked casts) arbeiten; die
  Dimensionsprüfung passiert bewusst zur Laufzeit statt im Typsystem.
- **Präfixe funktionieren beidseitig:** `MetricPrefix.KILO(JOULE)` → `kJ`,
  `SimpleUnitFormat.parse("kJ")` → Faktor 1000 auf J. Auch `km/h`, `µm`, `mN` parsen korrekt.
  **`mN` wird als Millinewton gelesen, nicht als m·N** — genau wie die Eindeutigkeitsregel aus
  F2 es verlangt. `min` ist die Minute, nicht `m·i·n`. Der Parser aus Phase 2 kann
  `SimpleUnitFormat` also als Longest-Match-Backend benutzen, statt die Präfix-Zerlegung
  selbst zu bauen.
- **Konvertierung:** `36 km/h` → `10 m/s` korrekt.
- **Genauigkeit:** `Quantities.getQuantity(BigDecimal(...), …)` rechnet mit `BigDecimal` weiter,
  `0.1 m + 0.2 m` → exakt `0.3 m`.
- **Aber:** Bei `to()` verengt Indriya den Zahlentyp, wo es verlustfrei geht — aus
  `BigDecimal("36") km/h` wird ein `java.lang.Integer` `10 m/s`. **Nie blind auf `BigDecimal`
  casten**, immer über `Number.toString()` gehen.
- Formelproben stimmen dimensional: `kg·m²/s²` = J, `W/A` = V, `V/A` = Ω, `m/m` = dimensionslos.

---

## Spike 3 — Symjas TeX-Export

Zugriff über `TeXFormFactory().convert(StringBuilder, IExpr)` (oder als Symja-Funktion
`TeXForm(expr)`).

### Brauchbar als Basis für den mathematischen Teil

| Eingabe | TeX-Output |
|---|---|
| `Sqrt(x)/3` | `\frac{\sqrt{x}}{3}` |
| `1/2*m*v^2` | `\frac{1}{2} \cdot m \cdot {v}^{2}` |
| `a/b/c` | `\frac{a}{b \cdot c}` |
| `D(x^2*Sin(x), x)` | `{x}^{2} \cdot \cos (x)+2 \cdot x \cdot \sin (x)` |
| `Integrate(x*Sin(x), x)` (ungeparst) | `\int  x \cdot \sin (x)\,\mathrm{d}x` |
| `D(f(x), x)` (ungeparst) | `\frac{\partial f(x)}{\partial x}` |
| `(3+4I)*(1-2I)` | `11 - 2\,i` |
| `Subscript(r, 1)` | `{r}_{1}` |
| `N(Pi, 50)` | `3.14159…751` (ohne Präzisionsmarker) |

Bemerkenswert: **`/` wird von Haus aus zu `\frac{}{}`, und `a/b/c` landet flach in einem
gemeinsamen Nenner** — das ist exakt die Auto-Bruch-Regel aus F2, geschenkt.

### Die vier Lücken

1. **Keine Einheiten-Semantik.** `2*kg*10*m/s^2` → `\frac{20 \cdot kg \cdot m}{{s}^{2}}`:
   Einheiten erscheinen als kursive Variablen, kein `\mathrm{}`, kein `\,` zwischen Zahl und
   Einheit, und die physikalische Anordnung *Zahl · Einheit* fehlt.
2. **Dezimaltrennzeichen ist der Punkt.** `19.62` statt `19{,}62` (F6).
3. **Exponentialschreibweise ist kein sauberes LaTeX:** `N(2^200, 20)` →
   `1.6069380442589902755*10^{60}` — nacktes `*` statt `\cdot`.
4. **Nur ausgewertete Ausdrücke rendern zuverlässig.** Der ungeparste, unausgewertete Ausdruck
   `11 - I*2` wird zu `11\left(  - i \right)  \cdot 2` — schlicht falsch. Ausgewertet ist
   derselbe Wert korrekt `11 - 2\,i`.

### Entscheidung

**Eigenbau-Serializer wie im Spec geplant — aber mit `TeXFormFactory` als Fallback für reine
Mathematik-Teilbäume.** Der Eigenbau muss ohnehin die Einheiten-, Komma- und
Schritt-Darstellung übernehmen; für Funktionen wie `\sin`, `\int`, `\partial` lohnt es nicht,
das nachzubauen.

### Zwei Bausteine für F5/F6, die aus dem Spike fallen

- **Variablen mit Index gehören als `Subscript(name, index)` in den AST**, nicht als
  zusammengesetzter Symbolname: `Subscript(r, 1)` → `{r}_{1}`, während das Symbol `r1`
  als literales `r1` gerendert wird. `Subscript(\[Omega], 0)` → `{\omega}_{0}` funktioniert ebenso.
- **`HoldForm` trägt den Schritt „eingesetzt, unausgewertet" aus F6.** `HoldForm(2*981/100)`
  bleibt bei `eval()` unausgewertet, und der Wrapper taucht im TeX nicht auf
  (→ `2 \cdot \frac{981}{100}`). Einschränkung: der Rationalzahl-Trick aus Spike 1 schlägt
  hier durch — im eingesetzten Schritt steht dann `\frac{981}{100}` statt `9{,}81`. Der
  Serializer braucht dafür eine eigene Dezimal-Formatierung (Rationalzahl mit
  Zehnerpotenz-Nenner → Dezimaldarstellung mit Komma).

---

## Offene Punkte für Phase 1

- Priorisierte `Dimension → benannte Einheit`-Tabelle anlegen (Seed: `Units.getInstance().units`,
  Konfliktfälle m/s vs. km/h, J vs. N·m entscheiden).
- `PhysicalQuantity` ungetypt über `Unit<*>` bauen, Dimensionsprüfung mit eigenen Fehlertexten.
- Präzision konsequent als `N(expr, precision)` durchreichen; Zahlen-Eingaben als Rationalzahlen halten.
- Eine `EvalEngine` pro Request, keine geteilte Singleton-Bean.
- Symja-Warmup beim Backend-Start (Integrate-Regelwerk) für N3.
