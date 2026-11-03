# Handheld features

Every feature the handheld has or is meant to have, one line each. The details are in the
documents this list is drawn from: [plans.md](../../plans/plans.md) §4–5,
[handheld-menus.md](../../design/handheld-menus.md) (*What is missing* and §7) and
[hardware.md](hardware.md). Ideas that are not planned yet are in
[feature-ideas.md](feature-ideas.md).

State on 2026-11-02.

- `[x]` works: on the board, or in the hui and the simulator where the board has no part in it
- `[~]` partly: what is missing follows the dash
- `[ ]` planned, not started

## Hardware

- [~] ESP32-P4 with 32 MB PSRAM: runs on the Waveshare dev kit; the product's own PCB is not built
- [x] ESP32-C6 as the radio, over ESP-Hosted (SDIO)
- [~] 3.5" 480×320 display: the ILI9486 RPi display on the dev kit; the product panel (maybe ST7796S) is not chosen
- [x] Resistive touch (XPT2046) on the display's SPI bus
- [ ] Keypad: about 53 keys on a TCA8418 over I²C; the layout and legend are not fixed
- [x] SD card slot (shares the SDMMC controller with the C6)
- [~] USB-C for flashing and power: the dev kit's port; the product's USB-C with ESD protection is not built
- [x] USB-A host port for a keyboard (dev kit)
- [ ] LiPo cell (606090, about 3000–4000 mAh)
- [ ] Charger with power path (BQ24074), charge LED, NTC check
- [ ] Buck-boost to 3.3 V (TPS63802)
- [ ] Power latch on the ON key (LTC2955): on, clean off, forced off when hung
- [ ] Fuel gauge (MAX17048)
- [ ] RTC with a backup supply from the cell (RV-3028)
- [ ] Backlight on its own 5 V boost, dimmed by PWM
- [ ] Enclosure and keycaps

## Power

- [~] Battery level in the top bar — drawn by the hui; no gauge on the board feeds it
- [~] Charging indicator (bolt, green fill) — drawn by the hui; the board does not set it yet
- [ ] Saves the session and switches off when the ON key is pressed
- [ ] Saves and switches off when the cell runs low
- [ ] Automatic off after 10 or 60 min (the SETUP entry exists, UI only)
- [ ] Backlight brightness 1–10 (the SETUP entry exists, UI only)
- [ ] Backlight dims after a pause, off after a longer one, back on with any key or touch
- [ ] Display sleep (`SLPIN`) while the screen is off
- [ ] CPU frequency scaling and automatic light sleep while idle
- [ ] Measured currents when off and in use

## System and boot

- [~] Near-instant boot (first screen within a few hundred ms) — the time to the first frame is logged; not yet measured from a cold start on the product
- [ ] Custom boot screen from flash
- [x] Engine on its own thread with a 64 KB stack; the screen, keys and touch stay live
- [x] Spinner in the top bar while the engine calculates
- [x] AC drops a running calculation
- [x] Per-calculation log over serial: mode, LaTeX, result, time, stack, heap
- [x] Settings, touch calibration and wallpaper kept in flash (NVS)
- [x] Zurücksetzen: settings, memory or everything, each confirmed with *Ja*
- [x] Firmware version shown in SETUP
- [x] The C6's firmware updated by the handheld at boot, tried once per image
- [ ] Firmware update over WLAN: search and install the latest GitHub release
- [ ] Firmware update from the desktop app (USB or the link)
- [ ] The release workflow attaches the firmware image
- [x] Stub engine build for fast UI work (`--no-default-features`)

## Display and look

- [x] Band-wise redraws (480×32 bands), only the bands that changed
- [~] Display colours, orientation and SPI clock — run on the board at 26.67 MHz; colours and orientation still to check by eye
- [x] Casio-like calculation screen: input top left, result bottom right
- [x] Top bar: mode name, significant digits, Exakt/Dez, D/R
- [x] Top bar: Wi-Fi fan (dim until connected), Bluetooth rune, *PRÜFUNG* tag
- [x] Top bar: busy ring for a remote calculation (set by the client once it exists)
- [ ] Top bar: clock
- [x] The desktop's purple palette; a muted colour per mode tile
- [x] Wallpaper from the SD card (BMP), with a preview in the list
- [x] Wallpaper behind the mode tiles, the menus or everything, with glass tiles over it
- [x] Brackets that grow with what they hold
- [x] 2D layout of fractions, roots, powers, integrals, sums, limits
- [~] Fonts — u8g2 bitmap fonts; Greek beyond π, μ, Ω is small; ∠, →, ⌊⌋, ⌈⌉, µ missing
- [ ] Font size normal/small (the SETUP entry exists, UI only)
- [x] German and English texts, German by default
- [ ] The step list (Rechenweg) on the result screen
- [ ] QR code of the result or the steps

## Input

- [x] Editor: a tree with a cursor, writing LaTeX
- [x] Casio-like templates: fraction, root, ⁿ√, power, subscript, log, ln, |x|, ∫, d/dx, Σ, lim
- [ ] Templates: nCr (`\binom`), conjugate (`\overline`), `cases`
- [x] History with ↑/↓, replaying an input
- [x] Ans
- [ ] PreAns (the core has it; no key yet)
- [x] Keys over the serial monitor
- [x] USB keyboard, German QWERTZ layout, key repeat
- [x] Touch: every menu by finger, taps, drag to scroll, page turns, − / + for numbers
- [x] Touch calibration: five crosses, least squares, redone when off by more than 20 px
- [ ] Keys still missing: CONST, CONV, ENG/←ENG, STO, VARS, CALC/SOLVE, PreAns, ∠, ∫, d/dx, Σ, x⁻¹, x³, 10ˣ, eˣ
- [ ] ALPHA letters on the keypad
- [ ] UNDO/REDO
- [ ] INS (wrap the previous item in the next template)
- [ ] Selection with SHIFT+←/→, copy and paste
- [ ] Text input for names and addresses on the keypad (today USB keyboard or serial)

## Modes

- [x] 1 Physik: numbers with units and constants, the default
- [x] 2 Algebra: simplify, factor, solve, systems
- [x] 3 Komplex: a+bi, complex roots
- [~] 4 Kurvendiskussion — plain inputs work; no screen per section, no Tangente/Normale, Fläche, Wertetabelle
- [x] 5 LGS: the matrix [A|b] up to 10 × 10, or typed equations
- [x] 6 Wertetabelle, with g(x) as a second column
- [ ] 7 Matrizen, with an editor (the core has the maths)
- [ ] 8 Basis-N (the core has the mode)
- [~] 9 Statistik, with a data editor — x̂ and ŷ for a given value are missing
- [x] 10 Verteilungen, one k or several
- [~] 11 Prüfen — the relation inserts work; the mode itself is not wired up
- [x] 12 Sitzungen (the tile opens the sessions menu)
- [ ] Later: Ungleichungen
- [ ] Later: Geometrie with vectors

## Results

- [x] Exact or decimal, as SETUP says
- [x] S⇔D between exact and decimal
- [x] Fractions improper or mixed (d/c, ab/c)
- [ ] S⇔D through all forms: recurring decimal, mixed fraction, °′″
- [ ] FACT (prime factors) from OPTN
- [ ] ENG and ←ENG on a result
- [ ] ▶r∠θ and ▶a+bi on a result
- [x] Number format Norm 1/2, Fix, Sci; prefixes; digit grouping (through the engine)
- [ ] The DOMAIN line for fractions in Algebra
- [ ] The quantity name in Physik
- [ ] Kurvendiskussion results, one page per section
- [x] LGS: the Gauss steps with their matrices

## Maths

What the handheld gets from `quantus-core`; `[~]` where the engine has it but the hui has no way
in yet.

- [x] Exact arithmetic, fractions, roots, π, e
- [x] Trigonometry and inverses in degrees or radians; ° and rad markers
- [x] sinh, cosh, tanh
- [~] arsinh, arcosh, artanh, coth — in the engine; the OPTN entries say *Noch nicht verfügbar*
- [x] ∫ with bounds (also infinite), d/dx at a point, Σ, lim (a⁺, a⁻, ∞)
- [~] Π (`\prod`) — in the engine; OPTN entry not wired
- [x] |x|, n!, mod, ⌊x⌋, ⌈x⌉, max, min, %
- [~] nCr, nPr, ggT, kgV, Int, Intg, Rnd, RndFix, ÷R — in the engine; no OPTN entries or template yet
- [~] Sexagesimal °′″ — in the engine; not in OPTN
- [x] Units after numbers, by quantity in OPTN (15 quantities)
- [x] All 24 prefixes, q to Q
- [~] Constants — in the engine (CODATA 2018, the Casio's 47); no CONST menu
- [~] ▶ unit conversion and the imperial units — in the engine; no CONV menu
- [x] Equations and systems, exact
- [x] Exact cubics and quartics, no [−100, 100] limit
- [~] SOLVE for a chosen variable — in the engine; no SOLVE screen
- [~] CALC (ask for the free variables, then substitute) — in the engine; no CALC prompt
- [ ] STO and VARS: store under any name, list, edit, delete
- [~] Basis-N, matrices, statistics, regressions, distributions — in the engine; no modes in the hui
- [~] wahr/falsch for an equation without unknowns — in the engine; Prüfen not wired
- [x] Step levels and the step settings passed to the engine (the list is not shown yet)

## Menus and settings

- [x] MODE: twelve tiles on two pages, digits 1–9, cursor and touch for all
- [x] SETUP (SHIFT+MODE): a submenu per entry, a list or a typed number with its range
- [x] SETUP Allgemein: Sprache, Winkeleinheit, Signifikante Stellen, Zahlenformat, Dezimalpräfixe, 1000er-Trennung
- [x] SETUP Ergebnis: exakt/dezimal, Brüche, Periodisch, Komplexe Zahlen, Quadratische Gleichungen
- [x] SETUP Rechenweg: level, Kürzungen durchstreichen, Gauß/Gauß-Jordan
- [x] SETUP Algebra: Binomformeln, Quadratische Ergänzung, Nenner, Identitäten mit Bedingung
- [x] SETUP Eingabe: a und y als Variablen, Buchstaben als Produkt, t/u/in als Einheiten, Unbekannte (LGS)
- [~] SETUP Gerät — Hintergrundbild, Speicherplatz, Zurücksetzen, Touch kalibrieren work; Helligkeit, Schriftgröße, Automatisch aus are UI only
- [~] SETUP Verbindungen — Prüfungsmodus, WLAN, Uhrzeit, installed version work; Rechnen im Netz, Sitzungen abgleichen, Firmware-Update, Bluetooth are UI only
- [x] OPTN per mode, inserting at the cursor
- [x] OPTN groups Analysis, Zahl, Hyperbol. Fkt., Winkel, Griechisch (the working entries)
- [x] OPTN Physik: Einheiten by quantity, Präfixe
- [x] OPTN Komplex: Argument, Realteil, Imaginärteil, Betrag
- [ ] OPTN Werkzeuge: CALC, SOLVE, ▶ Einheit
- [ ] OPTN for Kurvendiskussion, Matrizen, Basis-N
- [x] OPTN Wertetabelle, Statistik (Typ, Häufigkeit, rows), Verteilungen (Typ)
- [x] OPTN LGS: Eingabe, Zeilen, Spalten, Imaginäre Einheit, Alles löschen
- [ ] CONST and CONV menus by category
- [x] *Noch nicht verfügbar* for entries that wait for the engine or the firmware

## Storage and sessions

- [x] SD card: wallpapers in `/wallpapers/`, sessions in `/sessions/`
- [x] Speicherplatz: used, free, total, as a bar; *Keine SD-Karte* without one
- [x] Sessions: new, open, rename, duplicate, delete
- [x] New sessions named after the local time ("Sitzung 2026-11-02 14:30"), or numbered before the clock is set
- [x] The last session reopened at the start
- [x] The same session files as the desktop (exchange over the card)
- [ ] Session sync with the desktop over WLAN or Bluetooth
- [~] Mode inputs in the session — LGS, Wertetabelle (without g(x)), Statistik and Verteilungen
  are kept; Kurvendiskussion is not

## Connectivity

- [x] WLAN on/off, kept with the settings
- [x] Network scan with signal bars, a lock and a mark for saved ones
- [x] Password entry, shown as stars until S⇔D
- [x] Saved networks (up to 8), forget one
- [x] Joins the strongest saved network by itself, also after a restart
- [x] WLAN status: network, address, signal, or what the connection does
- [x] Network time (SNTP)
- [x] Clock set by hand: date and time
- [x] 14 named time zones with their summer time (EU, US, south-east Australia)
- [~] Prüfungsmodus — switches WLAN off and keeps it off; Bluetooth has no stack yet
- [ ] Bluetooth on/off
- [ ] Bluetooth pairing (desktop app, phone, keyboard), paired devices, visibility
- [ ] Bluetooth keyboard
- [ ] Remote engine: calculations over about 500 ms sent to a desktop or server, with an address setting
- [ ] Link to the desktop app with pairing
- [ ] The handheld as a keypad for the desktop app
- [ ] Phone companion (share results and sessions, set up WLAN)

## Development and testing

- [x] Simulator: the UI in a window on the PC, with the real engine
- [x] Mirror: the simulator's picture sent to the board over serial (`--mirror`)
- [x] Tour: a scripted walk through the UI (`--tour`, `tests/tour.rs`, the `tour` binary)
- [x] Bench binary on the board
- [x] Engine spike over serial (`firmware/esp32`)
- [ ] `:bench` run on the P4 against the WROOM numbers
