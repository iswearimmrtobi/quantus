# Quantus

SI-Einheiten-Rechner mit symbolischer Mathematik und LaTeX-Ein-/Ausgabe.
Spezifikation: [spec-si-rechner-v2.md](spec-si-rechner-v2.md).

**Aktueller Stand:** Phase 5 (Electron-Frontend) abgeschlossen — die Desktop-App ist
bedienbar. Getippt wird im MathLive-Editor, ausgewertet mit ⏎, und der Rechenweg kommt
dreistufig zurück:

```
m_1 = 2kg                       →   m_1 = 2\,\mathrm{kg}
F = m_1 \cdot 9{,}81\frac{m}{s^2}
    SYMBOLISCH   F = m_{1} \cdot 9{,}81\,\frac{\mathrm{m}}{\mathrm{s}^{2}}
    EINGESETZT   F = 2\,\mathrm{kg} \cdot 9{,}81\,\frac{\mathrm{m}}{\mathrm{s}^{2}}
    ERGEBNIS     F = 19{,}62\,\mathrm{N}
```

Auf dem Weg dorthin kam Phase 3 (Session-Variablen, Schritt-Erzeugung, REST-API) mit dazu —
ohne sie hätte das Frontend nichts zu reden gehabt. Davor Phase 2 (LaTeX rein und raus),
Phase 1 (Kern `PhysicalQuantity`) und Phase 0 (Fundament) mit den drei Spikes:
[docs/phase0-findings.md](docs/phase0-findings.md).

Code, Kommentare und Fehlermeldungen sind auf Englisch, die Projektdokumentation
(Spec, README, Findings) auf Deutsch — die Oberfläche ist deutsch.

## Voraussetzungen

JDK 21+ (die Gradle-Toolchain zieht sich Java 21 bei Bedarf selbst) und Node 20+ für die
Desktop-App.

## Befehle

```bash
./gradlew test        # Testsuite inkl. der Phase-0-Spikes (Spike-Ausgaben landen auf stdout)
./gradlew bootRun     # Backend allein auf http://127.0.0.1:8437
./gradlew bootJar     # Backend-Jar bauen — das startet die Desktop-App als Kindprozess
./gradlew build       # kompilieren, testen, Jar bauen

cd desktop
npm install           # einmalig (kopiert dabei die MathLive-Fonts nach public/fonts)
npm start             # Renderer bauen und App starten
npm run dev           # Vite-Dev-Server + Electron mit Hot Reload
npm run dist          # AppImage nach desktop/release/ bauen
```

Die App sucht beim Start ein laufendes Backend auf Port 8437 und hängt sich daran; findet
sie keins, startet sie `build/libs/quantus-*.jar` selbst als Kindprozess und beendet es
beim Schließen wieder. Für den ersten Start also einmal `./gradlew bootJar` laufen lassen.

## Paketierung

`npm run dist` baut ein AppImage (~215 MB) unter `desktop/release/`, das alles mitbringt:
Renderer, Backend-Jar und eine per jlink erzeugte Minimal-JRE (19 Module, ~56 MB). Auf dem
Zielrechner braucht es damit weder Java noch Node.

```bash
cd desktop && npm run dist
./release/Quantus-0.1.0-x86_64.AppImage
```

Der npm-Lauf ruft vorab `./gradlew desktopResources` auf; das baut Jar und JRE und prüft mit
`verifyRuntime`, ob das Backend unter der Minimal-JRE wirklich startet — fehlt ein Modul in
`backendModules` (build.gradle.kts), scheitert der Build hier statt später beim Anwender.

Das Backend beendet sich mit der App, auch wenn diese hart abgeschossen wird: Electron hält
dessen stdin offen, und der `ParentProcessWatchdog` macht aus dem EOF ein Shutdown-Signal.
Ohne den Schalter `--quantus.parent-watch=true` ist er aus — `bootRun` und der Docker-Betrieb
bleiben unberührt.

## Bedienung

| Taste | Wirkung |
|---|---|
| ⏎ | auswerten |
| Strg+N | neue Rechnung |
| Strg+J | Rechenweg ein-/ausblenden |
| Strg+, / Strg+H | Variablen- / Verlaufs-Panel |
| Strg+Shift+C | Ergebnis als LaTeX kopieren |
| Strg+Alt+C | ganze Rechnung als `align*`-Block kopieren |
| Esc | Panel schließen |

Im Eingabefeld gilt die übliche Formeleditor-Mechanik: `/` erzeugt einen Bruch, `*` ein
`\cdot`, `^` und `_` Hoch- und Tiefstellung. LaTeX-Befehle gehen auch direkt — `\frac`
tippen und mit ⏎ abschließen setzt einen Bruch mit zwei Platzhaltern, zwischen denen Tab
weiterspringt. Aus einem Index (und aus jeder anderen Klammer) führt die Pfeiltaste → wieder
heraus: `m_1` `→` `= 2kg` definiert die Variable.

Variablen tragen immer einen Index (`r_1`, `v_1`). Eine Eingabe mit Index-Label ist eine
Definition (`m_1 = 2kg`) und bleibt für die Session gültig; ein Label ohne Index (`F = …`)
benennt nur die Zeile.

## Struktur

```
src/main/kotlin/app/tobiasschmidt/quantus/
  cas/                                            Symja-Grenze: Präzision, exakte Zahlen
  units/                                          Indriya-Grenze: Lookup-Tabellen, Umrechnung
  quantity/                                       PhysicalQuantity — Wert + Einheit
  latex/                                          Lexer, Parser, AST, Serializer
  eval/                                           AST-Auswertung, Schritte, LatexCalculator
  session/                                        Session-Variablen, Auswertung im Session-Kontext
  web/                                            REST-API (/api/evaluate, /api/variables, /api/health)
src/main/resources/application.yaml               Port, Default-Präzision, CORS
src/test/kotlin/app/tobiasschmidt/quantus/spike/  Phase-0-Spikes (Symja, Indriya, TeX-Export)
desktop/electron/                                 Main-Prozess: Backend-Kindprozess, Menü, Preload
desktop/src/                                      React-Renderer: MathLive-Editor, Schritte, Panels
desktop/electron-builder.yml                      AppImage-Paketierung (Jar + jlink-JRE als Resources)
desktop/build/icon.svg                            App-Icon, daraus gerendert icon.png
docs/phase0-findings.md                           Erkenntnisse aus Phase 0
gradle/libs.versions.toml                         Versionen aller Abhängigkeiten
```

Die Modultrennung folgt N5: CAS-Logik (`cas`), Einheiten-Logik (`units`) und die
Vermittlungsschicht (`quantity`) hängen nur in eine Richtung voneinander ab; `web` kennt nur
`session` und `eval`.

Die Spikes sind bewusst Tests und keine Wegwerf-Mains: Sie halten das dokumentierte
Library-Verhalten fest, sodass ein Versionsupdate mit geändertem Verhalten auffällt.

## API

```
GET  /api/health                     → { status, application, precision }
POST /api/evaluate                   { latex, sessionId?, precision? }
                                     → { input, latex, result, steps[], alignBlock,
                                         value, unit, quantityName, dimension, variable }
GET  /api/variables?sessionId=…      → [ { key, symbol, latex, value, unit } ]
POST /api/variables                  { latex: "r_1 = 0{,}5m", … } → Variable
DELETE /api/variables/{key}          Variable vergessen
```

Fehler kommen als `400` mit `{ error, position }` zurück; `position` zeigt bei Parserfehlern
auf die Stelle in der Eingabe.
