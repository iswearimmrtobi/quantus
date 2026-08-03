# Quantus

SI-Einheiten-Rechner mit symbolischer Mathematik und LaTeX-Ein-/Ausgabe.
Spezifikation: [spec-si-rechner-v2.md](spec-si-rechner-v2.md).

**Aktueller Stand:** Phase 1 (Kern: `PhysicalQuantity`) abgeschlossen — Einheitenrechnung
mit Dimensionsprüfung, Vereinfachung auf benannte Einheiten und 50 Stellen Genauigkeit.
Davor Phase 0 (Fundament) mit den drei Spikes: [docs/phase0-findings.md](docs/phase0-findings.md).

Code, Kommentare und Fehlermeldungen sind auf Englisch, die Projektdokumentation
(Spec, README, Findings) auf Deutsch.

## Voraussetzungen

JDK 21+ (die Gradle-Toolchain zieht sich Java 21 bei Bedarf selbst).

## Befehle

```bash
./gradlew test        # Testsuite inkl. der Phase-0-Spikes (Spike-Ausgaben landen auf stdout)
./gradlew bootRun     # Backend auf http://localhost:8437
./gradlew build       # kompilieren, testen, Jar bauen
```

## Struktur

```
src/main/kotlin/app/tobiasschmidt/quantus/
  cas/                                            Symja-Grenze: Präzision, exakte Zahlen
  units/                                          Indriya-Grenze: Lookup-Tabellen, Umrechnung
  quantity/                                       PhysicalQuantity — Wert + Einheit
src/main/resources/application.yaml               Port, Default-Präzision
src/test/kotlin/app/tobiasschmidt/quantus/spike/  Phase-0-Spikes (Symja, Indriya, TeX-Export)
docs/phase0-findings.md                           Erkenntnisse aus Phase 0
gradle/libs.versions.toml                         Versionen aller Abhängigkeiten
```

Die Modultrennung folgt N5: CAS-Logik (`cas`), Einheiten-Logik (`units`) und die
Vermittlungsschicht (`quantity`) hängen nur in eine Richtung voneinander ab.

Die Spikes sind bewusst Tests und keine Wegwerf-Mains: Sie halten das dokumentierte
Library-Verhalten fest, sodass ein Versionsupdate mit geändertem Verhalten auffällt.
