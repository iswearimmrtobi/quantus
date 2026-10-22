# Quantus

SI-Einheiten-Rechner mit symbolischer Mathematik und LaTeX-Ein-/Ausgabe.
Spezifikation: [spec-si-rechner-v2.md](spec-si-rechner-v2.md).

**Aktueller Stand:** Die Engine ist von Kotlin/Spring auf Rust portiert, die Desktop-App läuft
auf Tauri. Getippt wird im MathLive-Editor, ausgewertet mit ⏎, und der Rechenweg kommt
dreistufig zurück:

```
m_1 = 2kg                       →   m_1 = 2\,\mathrm{kg}
F = m_1 \cdot 9{,}81\frac{m}{s^2}
    SYMBOLISCH   F = m_{1} \cdot 9{,}81\,\frac{\mathrm{m}}{\mathrm{s}^{2}}
    EINGESETZT   F = 2\,\mathrm{kg} \cdot 9{,}81\,\frac{\mathrm{m}}{\mathrm{s}^{2}}
    ERGEBNIS     F = 19{,}62\,\mathrm{N}
```

Die Rust-Engine liegt als Bibliothek in `crates/quantus-core` und wird von der Tauri-App
direkt aufgerufen — kein Server, kein Kindprozess. Die frühere Kotlin-Implementierung
(Phasen 0–5 der Spezifikation) ist entfernt; ihre Testfälle leben als Golden-Dateien in
`crates/quantus-core/tests/golden` weiter. Zum Hintergrund der Umstellung siehe
[migration-rust-tauri.md](migration-rust-tauri.md), zu den ursprünglichen Library-Spikes
[docs/phase0-findings.md](docs/phase0-findings.md).

Code, Kommentare und Fehlermeldungen sind auf Englisch, die Projektdokumentation
(Spec, README, Findings) auf Deutsch — die Oberfläche ist deutsch.

## Voraussetzungen

Rust (stable, Edition 2021) und Node 20+. Unter Linux braucht Tauri außerdem WebKitGTK 4.1
und die üblichen GTK-Entwicklungspakete (siehe die
[Tauri-Voraussetzungen](https://v2.tauri.app/start/prerequisites/)).

## Befehle

```bash
cargo test --workspace   # Engine-Tests inkl. Golden-Dateien und Tauri-IPC-Tests

cd desktop
npm install              # einmalig (kopiert dabei die MathLive-Fonts nach public/fonts)
npm run dev              # Vite-Dev-Server + Tauri-Fenster mit Hot Reload
npm run bundle           # AppImage und .deb bauen
npm run typecheck        # TypeScript prüfen
```

## Paketierung

`npm run bundle` ruft `tauri build` auf und legt AppImage und `.deb` unter
`target/release/bundle/` ab. Das AppImage (~100 MB) bringt WebKitGTK mit; das `.deb`
(~6 MB) hängt vom System-WebKitGTK ab. Der Desktop-Eintrag stammt aus
`desktop/src-tauri/packaging/quantus.desktop`, die Icons werden aus `desktop/build/icon.svg`
mit `npx tauri icon` erzeugt.

```bash
cd desktop && npm run bundle
./../target/release/bundle/appimage/Quantus_0.1.0_amd64.AppImage
```

## Bedienung

| Taste | Wirkung |
|---|---|
| ⏎ | auswerten |
| Strg+N | neue Rechnung |
| Strg+J | Rechenweg ein-/ausblenden |
| Strg+, / Strg+H | Variablen- / Verlaufs-Panel |
| Strg+Shift+C | Ergebnis als LaTeX kopieren |
| Strg+Alt+C | ganze Rechnung als `align*`-Block kopieren |
| `\` bzw. Strg+L | zwischen Formeleditor und LaTeX-Quelltext wechseln |
| Esc | Quelltext verlassen · Panel schließen |

### Zwei Eingabearten

Der **Formeleditor** ist die schnelle: `/` erzeugt einen Bruch, `*` ein `\cdot`, `^` und `_`
Hoch- und Tiefstellung, `2kg` eine Zahl mit Einheit. Aus einem Index (und aus jeder anderen
Klammer) führt die Pfeiltaste → wieder heraus — `m_1` `→` `= 2kg` definiert die Variable.

Der **Quelltext** ist die genaue: ein Backslash schaltet mitten im Tippen dorthin um, und ab
da steht da, was man schreibt — `\qty{12}{cm}`, `\sqrt[3]{27}`, `\frac{200\mathrm{km}}{2{,}5\mathrm{h}}`.
Unter dem Feld läuft die gesetzte Formel als Kontrolle mit. ⏎ rechnet, Esc geht zurück in
den Formeleditor; der Inhalt bleibt dabei in beide Richtungen erhalten.

Variablen tragen immer einen Index (`r_1`, `v_1`). Eine Eingabe mit Index-Label ist eine
Definition (`m_1 = 2kg`) und bleibt für die Session gültig; ein Label ohne Index (`F = …`)
benennt nur die Zeile.

## Struktur

```
crates/quantus-core/src/
  num.rs, quantity.rs                             Zahlen (dashu/astro-float) und PhysicalQuantity
  units/                                          Einheiten-Tabelle, Dimensionen, Umrechnung, Benennung
  latex/                                          Lexer, Parser, AST, Serializer
  eval/                                           AST-Auswertung, Substitution, Schritte
  session.rs, api.rs                              Session-Variablen und die JSON-Strukturen der App
crates/quantus-core/tests/                        Golden-Tests (aus den Kotlin-Tests extrahiert)
desktop/src-tauri/                                Tauri-Shell: Befehle, Menü, Single-Instance, Bundling
desktop/src/                                      React-Renderer: Formeleditor, LaTeX-Quelltext, Schritte, Panels
desktop/build/icon.svg                            App-Icon (Quelle für die Icon-Erzeugung)
docs/phase0-findings.md                           Erkenntnisse aus den Kotlin-Spikes (historisch)
migration-rust-tauri.md                           Plan und Befunde der Rust/Tauri-Umstellung
```

Die Modultrennung folgt N5: Zahlen, Einheiten und LaTeX-I/O hängen nur in eine Richtung
voneinander ab; die Tauri-Befehle kennen nur `api` und `SessionStore`.

## Schnittstelle

Der Renderer ruft Tauri-Befehle auf (`desktop/src/api.ts`), die `quantus_core::api` aufrufen:

```
evaluate         { request: { latex, sessionId?, precision? } }
                 → { input, latex, result, steps[], alignBlock,
                     value, unit, quantityName, dimension, variable }
define_variable  { request: { latex: "r_1 = 0{,}5m", … } } → Variable
list_variables   { sessionId? }               → [ { key, symbol, latex, value, unit } ]
delete_variable  { key, sessionId? }          Variable vergessen
```

Fehler kommen als `{ error, position }` zurück; `position` (0-basiert) zeigt bei
Parserfehlern auf die Stelle in der Eingabe.
