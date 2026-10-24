# Spec Sheet: SI-Einheiten-Rechner ("Quantus")

**Stand:** 2026-10-22 · **Status:** Phasen 0–5 umgesetzt, seit der Umstellung auf Rust + Tauri (2026-10) ohne JVM

> **Umstellung auf Rust + Tauri.** Die Abschnitte 3, 4 und 6 beschreiben den ursprünglichen
> Kotlin/Spring/Electron-Entwurf und sind als Entscheidungsverlauf stehen geblieben. Gültig ist
> heute: Engine als Rust-Bibliothek (`crates/quantus-core`, Zahlen mit `dashu`/`astro-float`,
> eigene Einheiten-Tabelle statt Indriya), Frontend als Tauri-App mit denselben React/MathLive-
> Quellen, Aufruf der Engine über Tauri-Befehle statt REST. Damit entfallen N2a (jlink-JRE) und
> vorerst N4 (self-hosted Service; ein dünnes axum-Crate über `quantus-core` bliebe möglich).
> Plan und Befunde: [migration-rust-tauri.md](migration-rust-tauri.md).

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
| N1 | ~~Backend in Kotlin + Spring Boot~~ — ersetzt: Engine in **Rust** (kein JavaScript-Backend) |
| N2 | Frontend als **Tauri-Desktop-App** (ursprünglich Electron) (TypeScript/React), MathLive als Eingabe-Widget; Anbindung ans Backend via REST |
| N2a | ~~Kotlin-Backend wird als lokaler Kindprozess mit der App ausgeliefert (gebündelte JRE via jlink); optional stattdessen Verbindung zu einer self-hosted Instanz~~ — entfällt, die Engine läuft im App-Prozess |
| N3 | Antwortzeit für typische Auswertungen < 200 ms |
| N4 | (zurückgestellt) Backend zusätzlich als self-hosted Service (Docker) betreibbar — gleiche Codebasis, Electron-App kann sich wahlweise dorthin verbinden |
| N5 | Saubere Trennung: CAS-Logik, Einheiten-Logik, LaTeX-I/O als getrennte Module |

---

## 4. Technologie-Entscheidungen

| Baustein | Wahl | Begründung |
|---|---|---|
| Backend | Kotlin + Spring Boot | Entscheidung, passt zu bestehendem Stack |
| CAS | **Symja** | Reines Java-CAS: komplexe Zahlen, Ableitungen, Integrale, arbitrary precision, Pattern Matching — nativ auf der JVM |
| Einheiten | **Indriya** (JSR-385 RI) | Dimensionsprüfung, Unit-Arithmetik, SI-System vollständig |
| Präzision | Symja intern (Apfloat) + BigDecimal an den Schnittstellen | Beliebige Genauigkeit |
| LaTeX-Parser | **Eigenbau** (recursive descent) | Kein reifes JVM-Pendant zu sympy vorhanden; benötigter Subset ist überschaubar |
| LaTeX-Serializer | **Eigenbau** (AST → LaTeX) | Volle Kontrolle über Schritt-Darstellung; Symjas TeX-Export ggf. als Basis prüfen |
| Persistenz | Spring Data JPA + **SQLite** (Datei im User-Profil) | History lokal, kein DB-Server nötig; PostgreSQL nur bei Self-hosted-Betrieb |
| Frontend | **Electron** + TypeScript/React + **MathLive** | Desktop-App, LaTeX-Eingabe mit physischer Tastatur/Numpad |
| Packaging | electron-builder + jlink-JRE für das Kotlin-Backend | Backend startet als Kindprozess auf localhost, App bleibt Doppelklick-installierbar |

### Architektur-Kernentscheidung: Symja ⟷ Indriya

Es existiert **keine fertige Brücke** zwischen beiden Libraries. Daher eigene Vermittlungsschicht:

```
PhysicalQuantity = (Symja IExpr: Zahlwert)  +  (Indriya Unit: Dimension)
```

- **Symja** rechnet alles Numerische/Symbolische (Wert, komplexe Zahlen, Analysis)
- **Indriya** führt parallel die Dimensions-Buchhaltung (Unit-Arithmetik bei ×/÷, Kompatibilitätsprüfung bei +/−)
- Die Vermittlungsschicht mappt jede Operation auf beide Seiten

```
┌─────────────────────────────────────────────┐
│           Electron-App (TS/React)           │
│         MathLive-Editor · Numpad · History  │
│   startet Backend als Kindprozess (jlink)   │
└──────────────────┬──────────────────────────┘
                   │ REST auf localhost (JSON: LaTeX rein, Schritte+LaTeX raus)
┌──────────────────▼──────────────────────────┐
│           Spring Boot (Kotlin)              │
│  ┌─────────────┐  ┌──────────────────────┐  │
│  │ LaTeX-Parser│→ │  Eval-Orchestrierung │  │
│  └─────────────┘  │  (Schritt-Erzeugung) │  │
│  ┌─────────────┐  └──────┬───────────────┘  │
│  │LaTeX-Serial.│←────────┤                  │
│  └─────────────┘  ┌──────▼───────────────┐  │
│                   │  PhysicalQuantity    │  │
│                   │  Symja ⟷ Indriya     │  │
│                   └──────────────────────┘  │
│  ┌───────────────────────────────────────┐  │
│  │ History (Spring Data JPA)             │  │
│  └───────────────────────────────────────┘  │
└─────────────────────────────────────────────┘
```

---

## 5. API-Skizze

```
POST /api/evaluate
  { "latex": "...", "sessionId": "...", "precision": 50 }
  → { "steps": [ { "kind": "symbolic"|"substituted"|"result",
                   "latex": "..." } ],
      "value": "...", "unit": "N", "error": null }

POST /api/variables          # Variable definieren
GET  /api/variables          # Session-Variablen abrufen
GET  /api/history?query=...  # Historie
POST /api/derive | /api/integrate   # oder als LaTeX-Kommando \frac{d}{dx} im Eval-Endpoint
```

---

## 6. Roadmap

### Phase 0 — Fundament *(~1 Wochenende)* — **erledigt (2026-08-03)**
- [x] Gradle-Projekt: Kotlin, Spring Boot, Symja, Indriya, Testframework
- [x] Spike: Symja-API kennenlernen (Ausdruck bauen, auswerten, Präzision setzen)
- [x] Spike: Indriya-Unit-Arithmetik (kg × m/s² → prüfen, ob/wie N erkannt wird)
- [x] Spike: Symjas eingebauten TeX-Export testen — brauchbar oder Eigenbau?
- **Meilenstein:** Beide Libraries laufen im selben Projekt, Erkenntnisse dokumentiert
  → siehe [phase0-findings.md](phase0-findings.md)

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

**Offene Punkte aus Phase 2:** Variablen löst inzwischen Phase 3 auf; Klammern im Output werden aus der Präzedenz neu
abgeleitet statt aus der Eingabe übernommen; Temperaturen mit eigenem Nullpunkt (°C) lassen
sich bewusst nicht mit K addieren, sondern nur explizit umrechnen.

### Phase 3 — Variablen & Schritte *(~1–2 Wochen nebenbei)* — **erledigt (2026-08-03)**
- [x] Session-Kontext mit Variablendefinitionen (`r_1 = 0,5 m`) — Label **mit** Index definiert,
  Label ohne Index benennt nur die Zeile
- [x] Schritt-Erzeugung: symbolisch → eingesetzt → Ergebnis (jeweils als LaTeX); der
  Einsetz-Schritt entfällt, wo es nichts einzusetzen gibt
- [x] REST-API (`/api/evaluate`, `/api/variables`, `/api/health`) + Session-Handling
- **Meilenstein:** Physikaufgabe mit Variablen liefert dreistufigen LaTeX-Rechenweg via API

**Offen aus Phase 3:** Sessions leben nur im Speicher (Persistenz ist Phase 6); der
Einsetz-Schritt zeigt Dezimalwerte, keine exakten Brüche.

### Phase 4 — Analysis & komplexe Zahlen *(~1 Woche nebenbei)*
- [ ] Ableitungen und unbestimmte Integrale über Symja anbinden (`\frac{d}{dx}`, `\int`)
- [ ] Komplexe Zahlen: Parsing (i), Ausgabeformen (kartesisch/polar)
- [ ] Klären: Einheiten in Analysis-Ausdrücken (v(t) ableiten → Einheit /s mitführen?) — ggf. zunächst einheitenlos
- **Meilenstein:** `\frac{d}{dx} x^2 \sin(x)` und `(3+4i)·(1−2i)` funktionieren

### Phase 5 — Electron-Frontend *(~2–3 Wochen nebenbei)* — **erledigt (2026-08-03)**
- [x] Electron-Grundgerüst (TypeScript/Vite/React), Backend-Kindprozess-Start mit Health-Check und sauberem Shutdown
  (ein bereits laufendes Backend wird erkannt und nur mitbenutzt, nicht beendet)
- [x] MathLive-Editor mit Live-Rendering, darunter der LaTeX-Quelltext zum Kopieren
- [x] Zweite Eingabeart für F2: ein Backslash (oder Strg+L) schaltet auf reinen
  LaTeX-Quelltext um, mit gesetzter Formel als Kontrolle darunter — der Formeleditor kann
  nur, was er als Struktur kennt, getippt werden muss aber alles können (`\qty{12}{cm}`,
  `\sqrt[3]{27}`)
- [x] Numpad-/Tastatur-Workflow: Enter = auswerten, Kürzel für Copy-LaTeX (globale Shortcuts als Menü-Accelerators)
- [x] Schritt-Ansicht mit Copy-Buttons (einzeln + gesamt als `align*`), Clipboard via Electron-API
- [x] Variablen-Panel (Klick fügt die Variable ein, ✕ vergisst sie) plus Verlaufs-Panel für die laufende Sitzung
- **Meilenstein:** Kompletter Workflow in der Desktop-App bedienbar ohne Maus

**Offen aus Phase 5:** Der Verlauf lebt nur im Renderer (Persistenz ist Phase 6); die
Analysis-Eingaben aus Phase 4 fehlen noch, die Oberfläche hat dafür keine eigenen Elemente.

### Phase 6 — History & Packaging *(~1–2 Wochen nebenbei)*
- [ ] Persistenz der Rechnungen + Variablen (JPA + SQLite im User-Profil)
- [ ] History-UI: laden, erneut ausführen
- [x] Packaging: jlink-JRE (19 Module, ~56 MB) + Backend-Jar gebündelt, electron-builder baut
  ein AppImage für Linux — Windows-Installer offen
- [ ] Optional: Dockerfile fürs Backend, Betrieb im Homelab mit Remote-Verbindung aus der App
- **Meilenstein:** v1.0 als installierbare Desktop-App im Einsatz

### Backlog (nach v1.0)
- Bestimmte Integrale, Gleichungslösen, Einheiten-Zielkonvertierung im Output ("Ergebnis in kWh")
- History-Suche, Export ganzer Rechnungen als .tex-Datei
- Physikalische Konstanten-Bibliothek (c, h, G, …) als vordefinierte Variablen

---

## 7. Risiken

| Risiko | Auswirkung | Gegenmaßnahme |
|---|---|---|
| LaTeX-Parser wird unterschätzt | Verzögerung Phase 2 | Subset klein halten, ANTLR als Fallback |
| Mehrdeutigkeiten bei markuploser Einheiten-Erkennung (Präfix-Zerlegung: mN, min, …) | Falsch geparste Eingaben | Feste Regeln: Longest-Match gegen Einheiten-Tabelle vor Präfix-Zerlegung; implizite Einheitenprodukte verboten (immer `\cdot`); Testsuite mit allen bekannten Kollisionsfällen |
| Indriya erkennt abgeleitete Einheiten nicht automatisch als N/J/… | F1 unvollständig | Eigene Dimension→Name-Lookup-Tabelle (überschaubar: ~20 SI-Einheiten) |
| Symja-TeX-Export unbrauchbar für Schritt-Darstellung | Mehraufwand Serializer | Eigenbau ohnehin eingeplant, Export nur als Abkürzung |
| Einheiten + Analysis kombiniert ist konzeptionell schwierig | F8 eingeschränkt | Analysis zunächst einheitenlos ausliefern (Phase 4) |
| JVM-Bündelung bläht die App auf / Prozess-Lifecycle fummelig | Installer groß (~100–150 MB), Zombie-Prozesse | **eingetreten und behandelt (Phase 6):** jlink-Minimal-Runtime mit 19 Modulen, AppImage trotzdem ~215 MB; Backend hängt am Electron-Lifecycle (Health-Check beim Start, Kill on quit) und beendet sich zusätzlich selbst, wenn seine stdin-Pipe zugeht — sonst überlebt es ein `kill -9` auf die App |
