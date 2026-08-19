# Quantus

SI-Einheiten-Rechner mit symbolischer Mathematik und LaTeX-Ein-/Ausgabe.
Spezifikation: [spec-si-rechner-v3.md](spec-si-rechner-v3.md).

**Aktueller Stand:** Phase 4 (Analysis & komplexe Zahlen) abgeschlossen. `EvalFacade` kann
neben Einheitenrechnung auch Ableitungen, unbestimmte Integrale und komplexe Zahlen:

```
EvalFacade.derive("x^2\sin(x)", "x")     →  x²cos(x) + 2x·sin(x)
EvalFacade.integrate("x\sin(x)", "x")    →  -x·cos(x) + sin(x) + C
EvalFacade.evaluate("(3+4i)(1-2i)", …)   →  11 - 2i   (wahlweise polar: r·e^{iφ})
```

Analysis-Ausdrücke sind vorerst einheitenlos (Spec Abschnitt 6); ein bare Bezeichner wie `x`
braucht dort — anders als bei physikalischen Größen (F2) — keinen Index.

Davor Phase 3 (Variablen & Schritte, `EvalFacade` mit Sessions und dreistufigem LaTeX-Rechenweg),
Phase 2 (LaTeX rein und raus, `LatexCalculator`), Phase 1 (Kern `PhysicalQuantity`:
Einheitenrechnung mit Dimensionsprüfung, Vereinfachung auf benannte Einheiten, 50 Stellen)
und Phase 0 (Fundament) mit den drei Spikes: [docs/phase0-findings.md](docs/phase0-findings.md).

Code, Kommentare und Fehlermeldungen sind auf Englisch, die Projektdokumentation
(Spec, README, Findings) auf Deutsch.

## Voraussetzungen

JDK 21+ (die Gradle-Toolchain zieht sich Java 21 bei Bedarf selbst).

## Befehle

```bash
./gradlew test        # Testsuite inkl. der Phase-0-Spikes (Spike-Ausgaben landen auf stdout)
./gradlew run         # App starten (ein einzelner JVM-Prozess, siehe N2a)
./gradlew build       # kompilieren, testen, Jar bauen
```

## Struktur

```
src/main/kotlin/app/tobiasschmidt/quantus/
  cas/                                            Symja-Grenze: Präzision, exakte Zahlen
  units/                                          Indriya-Grenze: Lookup-Tabellen, Umrechnung
  quantity/                                       PhysicalQuantity — Wert + Einheit
  latex/                                          Lexer, Parser, AST, Serializer
  eval/                                           AST-Auswertung, Sessions, EvalFacade (Schritte)
src/test/kotlin/app/tobiasschmidt/quantus/spike/  Phase-0-Spikes (Symja, Indriya, TeX-Export)
docs/phase0-findings.md                           Erkenntnisse aus Phase 0
gradle/libs.versions.toml                         Versionen aller Abhängigkeiten
```

Die Modultrennung folgt N5: CAS-Logik (`cas`), Einheiten-Logik (`units`) und die
Vermittlungsschicht (`quantity`) hängen nur in eine Richtung voneinander ab.

Die Spikes sind bewusst Tests und keine Wegwerf-Mains: Sie halten das dokumentierte
Library-Verhalten fest, sodass ein Versionsupdate mit geändertem Verhalten auffällt.
