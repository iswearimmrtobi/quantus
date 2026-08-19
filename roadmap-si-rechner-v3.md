# Roadmap: SI-Einheiten-Rechner ("Quantus")

**Stand:** 2026-08-18 · **Status:** Konzeptphase (Tech-Stack-Revision)

Ausgelagert aus [spec-si-rechner-v2.md](spec-si-rechner-v2.md) Abschnitt 6, aktualisiert für den
in [spec-si-rechner-v3.md](spec-si-rechner-v3.md) beschriebenen Tech-Stack (JavaFX/ein JVM-Prozess
statt Electron + Spring Boot). Phase 0–4 sind inhaltlich unverändert — sie betreffen ausschließlich
die Kernlogik, nicht die UI-Architektur.

---

### Phase 0 — Fundament *(~1 Wochenende)* — **erledigt (2026-08-03)**
- [x] Gradle-Projekt: Kotlin, Spring Boot, Symja, Indriya, Testframework
- [x] Spike: Symja-API kennenlernen (Ausdruck bauen, auswerten, Präzision setzen)
- [x] Spike: Indriya-Unit-Arithmetik (kg × m/s² → prüfen, ob/wie N erkannt wird)
- [x] Spike: Symjas eingebauten TeX-Export testen — brauchbar oder Eigenbau?
- **Meilenstein:** Beide Libraries laufen im selben Projekt, Erkenntnisse dokumentiert
  → siehe [docs/phase0-findings.md](docs/phase0-findings.md)

### Phase 1 — Kern: PhysicalQuantity *(~1–2 Wochen nebenbei)* — **erledigt (2026-08-03)**
- [x] `PhysicalQuantity`-Klasse (IExpr + Unit) mit +, −, ×, ÷, ^, √
- [x] Dimensionsprüfung mit verständlichen Fehlermeldungen
- [x] Vereinfachung auf benannte Einheiten (kg·m/s² → N); ggf. eigene Lookup-Tabelle
- [x] Präzisions-Konfiguration durchgereicht bis Symja
- [x] Unit-Tests: Standard-Physikaufgaben als Testfälle (F=ma, E=½mv², U=RI, …)
- **Meilenstein:** `1 kg × 1 m/s² = 1 N` programmatisch korrekt, mit 50 Stellen

### Phase 2 — LaTeX rein und raus *(~2 Wochen nebenbei)* — **erledigt (2026-08-03)**
- [x] Recursive-descent-Parser: Zahlen (inkl. dt. Komma), `+ − \times \cdot / \frac ^ \sqrt`, Klammern, Variablen mit Subskript
- [x] **Einheiten-Erkennung ohne Markup:** Tokenizer klassifiziert nackte Bezeichner über die Eindeutigkeitsregel — mit Subskript = Variable, ohne Subskript = Lookup in Einheiten-Tabelle (inkl. Präfix-Zerlegung: `kJ` → kilo + Joule), sonst Konstanten-Tabelle, sonst Parserfehler. `2kg` = implizite Multiplikation Zahl × Einheit
- [x] Sonderfälle der Präfix-Zerlegung testen: Mehrdeutigkeiten wie `mN` (Millinewton, nie Meter·Newton — implizite Einheitenprodukte erfordern `\cdot`), `min` vs. `m·i·n`, `Pa` vs. Präfix-Deutung
- [x] `\mathrm{}`/`\text{}` weiterhin als explizite Einheiten-Markierung (dazu siunitx `\qty{}{}`, `\si{}`)
- [x] **Auto-Bruch-Umbau:** `/`-Operator immer als `\frac{}{}` rendern, Operandengrenzen nach Präzedenz, Ketten-Vereinfachung (`a/b/c` → `\frac{a}{b\cdot c}`) als Normalisierungsschritt nach dem Parsen
- [x] AST → PhysicalQuantity-Auswertung
- [x] LaTeX-Serializer: AST und Ergebnisse zurück nach LaTeX (Einheiten im Output normalisiert als `\mathrm{}`)
- [x] Fehlertoleranz: Parserfehler mit Positionsangabe
- **Meilenstein:** Round-Trip für `F = 2kg \cdot 10\frac{m}{s^2}` → `F = 20\,\mathrm{N}`

**Offene Punkte aus Phase 2 (für Phase 3):** Variablen im AST werden erkannt, aber noch nicht
aufgelöst (`UnresolvedVariableException`); Klammern im Output werden aus der Präzedenz neu
abgeleitet statt aus der Eingabe übernommen; Temperaturen mit eigenem Nullpunkt (°C) lassen
sich bewusst nicht mit K addieren, sondern nur explizit umrechnen.

### Phase 3 — Variablen & Schritte *(~1–2 Wochen nebenbei)* — **erledigt (2026-08-19)**
- [x] Session-Kontext mit Variablendefinitionen (`r_1 = 0,5 m`)
- [x] Schritt-Erzeugung: symbolisch → eingesetzt → Ergebnis (jeweils als LaTeX)
- [x] Interne Eval-Fassade (`evaluate`, `defineVariable`, `getVariables` — siehe
      [spec-si-rechner-v3.md](spec-si-rechner-v3.md) Abschnitt 5) + Session-Handling
- **Meilenstein:** Physikaufgabe mit Variablen liefert dreistufigen LaTeX-Rechenweg über die interne Fassade

### Phase 4 — Analysis & komplexe Zahlen *(~1 Woche nebenbei)* — **erledigt (2026-08-19)**
- [x] Ableitungen und unbestimmte Integrale über Symja anbinden (`\frac{d}{dx}`, `\int`)
- [x] Komplexe Zahlen: Parsing (i), Ausgabeformen (kartesisch/polar)
- [x] Klären: Einheiten in Analysis-Ausdrücken (v(t) ableiten → Einheit /s mitführen?) — ggf. zunächst einheitenlos
  → entschieden: Analysis-Ausdrücke bleiben vorerst einheitenlos, `EvalFacade.derive`/`integrate`
  weisen eine Einheit im Ausdruck explizit zurück, statt sie stillschweigend zu ignorieren
- **Meilenstein:** `\frac{d}{dx} x^2 \sin(x)` und `(3+4i)·(1−2i)` funktionieren

### Phase 5 — JavaFX-Frontend *(~2–3 Wochen nebenbei)*
- [ ] JavaFX-Grundgerüst (Kotlin), Fensteraufbau/Layout nach `quantus-ui-v2.html`-Mockup (Dark Theme, IBM Plex Sans/JetBrains Mono)
- [ ] MathLive-Editor eingebettet in eine JavaFX-`WebView`; schlanke JS↔Kotlin-Bridge (LaTeX-Wert + Cursor-Events, siehe Risiko in der Spec)
- [ ] Numpad-/Tastatur-Workflow: Enter = auswerten, Kürzel über JavaFX-Accelerators (kein separater Electron-Global-Shortcut-Mechanismus mehr nötig)
- [ ] Schritt-Ansicht (nativ, JavaFX/CSS) mit Copy-Buttons (einzeln + gesamt als `align*`), Zwischenablage über die JavaFX-Clipboard-API
- [ ] Variablen-Panel (nativ)
- **Meilenstein:** Kompletter Workflow im Programm bedienbar ohne Maus, in einem einzigen JVM-Prozess

### Phase 6 — History & Packaging *(~1–2 Wochen nebenbei)*
- [ ] Persistenz der Rechnungen + Variablen (Exposed/JDBC + SQLite im User-Profil)
- [ ] History-UI: laden, erneut ausführen
- [ ] Packaging: `jpackage`-Installer pro Zielplattform (Windows/Linux), gebündelte Minimal-JRE — ein Gradle-Task statt jlink+electron-builder
- [ ] Optional (Backlog): dünne Ktor/Javalin-Fassade für das Self-hosted-Szenario (N4)
- **Meilenstein:** v1.0 als installierbares Programm im Einsatz

### Backlog (nach v1.0)
- Bestimmte Integrale, Gleichungslösen, Einheiten-Zielkonvertierung im Output ("Ergebnis in kWh")
- History-Suche, Export ganzer Rechnungen als .tex-Datei
- Physikalische Konstanten-Bibliothek (c, h, G, …) als vordefinierte Variablen
- Self-hosted-Betrieb über eine dünne Ktor/Javalin-Fassade vor den bestehenden Kernmodulen (siehe N4)
