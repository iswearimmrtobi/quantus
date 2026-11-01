# Handheld menus

The menu structure for the handheld (`firmware/ui`, the hui), modelled on the Casio fx-991DE X.

- **What is missing** (below): the open work, by where it goes.
- **Section 1–3**: the sources, and the Casio's menus as they are.
- **Section 4**: each Casio entry mapped to Quantus, with the decision taken for it.
- **Section 5**: what Quantus can do that the Casio cannot.
- **Section 6**: sessions.
- **Section 7**: the merged tree the hui implements, with its status.
- **Section 8**: where the work goes, mostly into `quantus-core`, for the desktop as well.
- **Section 9**: open questions.

The keypad is "modelled on the Casio fx-DE line, not a copy"
([calculator-hardware.md](calculator-hardware.md) §4) and its legend is not fixed yet
([plans.md](../plans/plans.md) §4).

## What is missing

Compared with the code on 2026-11-01. **Core** is `quantus-core` (the desktop and the hui get it
alike), **hui** the handheld UI in `firmware/ui` (with the board firmware where noted),
**Desktop** the desktop UI. A hui or desktop entry often waits for the core entry above it.

### Core (`quantus-core`)

- [x] Angle unit: setting `angleUnit` Grad/Bogenmaß; ° and rad mark a value regardless of it
- [ ] Sexagesimal °′″: input and result form
- [ ] Number format: Norm 1/2, Fix 0–9, Sci 0–9 in the formatter
- [ ] Dezimalpräfixe on plain numbers, and ENG / ←ENG steps of three
- [ ] Digit grouping (1000er-Trennung) with a thin space
- [ ] Result forms carried by each result: mixed fractions (`fractionForm` ab/c, d/c),
      recurring decimals (`recurring`) and their input, prime factors (FACT)
- [ ] Complex polar form: `complexForm` a+bi / r∠θ, ▶r∠θ / ▶a+bi, ∠ input
- [ ] Basis-N mode `baseN`: integers, DEC/HEX/BIN/OCT, d/h/b/o, and/or/xor/xnor/Not/Neg
- [ ] Matrices: values with + − ×, inverse, transpose, identity, det, powers, REF/RREF, exact,
      with steps
- [ ] Stochastik: lists, one-variable statistics, the seven regressions, the distributions
- [ ] Functions: arsinh, arcosh, artanh, coth, `\prod`, nPr, ggT, kgV, Int, Intg, Rnd, RndFix, ÷R
- [ ] Constants: the 32 missing ones (CODATA 2018), in the Casio's six categories, usable in
      every mode
- [ ] Units: the ▶ unit operator, and the missing imperial units (ft, yd, lb, oz, gal, psi, Btu,
      hp, °F …)
- [ ] Exact cubics and quartics (Cardano, Ferrari); drop the [−100, 100] limit
- [ ] API: the free variables of an expression (CALC), solve for a named variable (SOLVE)
- [ ] wahr/falsch for an equation without unknowns, and a Prüfen mode with chaining
- [ ] Sessions: history, mode inputs, name and timestamps in `session.rs`, serde JSON with a
      format version
- [ ] PreAns next to Ans in the session
- [ ] Remote engine: the request and response format (the JSON of `api.rs`)
- [ ] Later: polynomial inequalities; vectors with Geometrie

### Handheld UI (`firmware/ui` and the board)

- [ ] Kurvendiskussion: a screen per section, Tangente/Normale, Fläche, Wertetabelle
- [ ] LGS: the matrix editor [A|b] and its size
- [ ] Wertetabelle mode, with g(x) as a second column
- [ ] The modes Matrizen (with editor), Basis-N, Statistik (with editor), Verteilungen, Prüfen
- [ ] The step list (Rechenweg) on the result screen
- [ ] S⇔D through all forms of a result; OPTN for FACT, ENG and the polar form
- [ ] The ◌ SETUP entries once the core has them: Winkeleinheit, Zahlenformat,
      Dezimalpräfixe, 1000er-Trennung, Brüche, Periodisch, Komplexe Zahlen
- [ ] The ◌ OPTN entries once the core has them (section 7.3)
- [ ] Editor templates: nCr (`\binom`), Konjugierte (`\overline`), `cases`
- [ ] Font glyphs: ∠, →, ⌊⌋, ⌈⌉, µ
- [ ] Keys: CONST, CONV, ENG/←ENG, STO, VARS, CALC/SOLVE, Sitzungen, PreAns, ∠, ∫, d/dx, Σ,
      x⁻¹, x³, 10ˣ, eˣ, and the ALPHA legend (section 7.4)
- [ ] CONST and CONV menus by category
- [ ] CALC prompt and SOLVE screen
- [ ] VARS screen: insert, edit, delete session variables
- [ ] Editing: UNDO/REDO, INS, selection with SHIFT+←/→, copy and paste
- [ ] Sitzungen: the menu, files on the SD card, the last session reopened on wake-up
- [ ] Gerät: brightness, font size, auto-off (firmware)
- [ ] QR code of the result or the steps
- [ ] Prüfungsmodus actually switching the radios off
- [ ] WLAN: on/off, networks, saved networks, status
- [ ] Bluetooth: on/off, pairing, paired devices, visibility
- [ ] Remote engine client and its Rechnen im Netz settings
- [ ] Session sync with the desktop
- [ ] Clock: NTP or by hand, time zone
- [ ] Firmware update: search and install over WLAN, receive an image from the desktop
- [ ] Measure deep sleep on the real board (section 6)

### Desktop UI (`desktop/`)

- [ ] Settings dialog for the new settings (angle unit, number format, prefixes, fraction form,
      complex form, recurring decimals, digit grouping)
- [ ] A form switch on the result; ENG on a result
- [ ] The modes Basis-N, Matrizen (input and output), Stochastik (with a data editor), Prüfen
- [ ] A constants and units picker
- [ ] SOLVE and CALC in the input's context menu
- [ ] Sessions: one file per session in the config directory, the session menu, the last one
      opened on start
- [ ] A listener the handheld reaches over WLAN and Bluetooth, with pairing
- [ ] A handheld updater: the image from the GitHub releases, uploaded over USB or the link
- [ ] The release workflow attaches the firmware image ([RELEASING.md](../RELEASING.md))

## 1. Sources

- **Casio**: *fx-87DE X / fx-991DE X Bedienungsanleitung*, RJA532713-001V01 (© 2015), the
  German manual from `support.casio.com/storage/de/manual/pdf/DE/004/fx-87_991DEX_DE.pdf`
  (read from the Wayback Machine copy; the Casio site returned 503). Page numbers below are the
  manual's. Entries marked "fx-991DE X only" in the manual are included; this is the 991.
  The manual prints the keys as icons, so a key path here names the function as printed in
  brackets, for example *(SETUP)*, and not its SHIFT/ALPHA modifier.
- **Quantus**: `docs/docs/guides/*`, `docs/docs/reference/{syntax,settings,units-and-constants,limits}.md`,
  [analysis-mode.md](analysis-mode.md), [electronics-module.md](electronics-module.md),
  `plans/plans.md`, `firmware/ui/src/{app,key}.rs`.

Labels are quoted in German as the Casio shows them, with English where it helps; the firmware
needs both (`desktop/src/i18n/en.ts`, `de.ts` are the model).

## 2. How the Casio's menus work (p. 5)

| Key | Opens |
|---|---|
| MENU | the main menu (Hauptmenü): mode icons, chosen with the cursor keys and OK/=, or the digit shown |
| SHIFT + MENU | *(SETUP)* |
| OPTN | the mode's option menu; its content depends on the mode |
| *(CONST)* | scientific constants, by category |
| *(CONV)* | metric conversions, by category |
| *(RESET)* | Setupdaten / Speicher / Alle initialis. |
| *(RECALL)*, STO | the variables A–F, M, x, y |
| CALC, *(SOLVE)* | evaluate an expression for given values / solve it numerically |
| S⇔D | switch the result between fraction, √, π form and decimal |
| *(≈)* | evaluate straight to a decimal |
| *(QR)* | a QR code of the current screen |

In every menu: a digit picks the numbered entry, ↑/↓ scroll when there is a scroll bar, ← goes
back from a submenu (shown by a left arrow), AC closes the menu without choosing.

## 3. The Casio fx-991DE X

### 3.1 Main menu (p. 5)

| # | Icon | Purpose |
|---|---|---|
| 1 | Berechnungen | general calculation (the default) |
| 2 | Komplexe Zahlen | complex numbers |
| 3 | Basis-N | binary, octal, decimal, hexadecimal |
| 4 | Matrizen | matrices up to 4×4 |
| 5 | Vektorrechnung | vectors in 2D and 3D |
| 6 | Statistik | statistics and regression |
| 7 | Verteilungsfkt. | probability distributions |
| 8 | Tabellenkalk. | spreadsheet, 45 rows × 5 columns |
| 9 | Tabellen | value table of f(x) and g(x) |
| 10 | Gleichung/Funkt | systems of linear equations, polynomial equations |
| 11 | Ungleichungen | polynomial inequalities |
| 12 | Berechn prüf | verify (in)equalities: true or false |
| 13 | Verhältnisse | ratios A/B = X/D |

### 3.2 SETUP (pp. 7–8)

◆ marks the default.

- **Eingabe/Ausgabe**: Math → Math◆ · Math → Dezim. · Lin. → Linear · Lin. → Dezim.
- **Winkeleinheit**: Gradmaß (D)◆ · Bogenmaß (R) · Gon (G)
- **Zahlenformat**: Fix (0–9 decimal places) · Sci (0–9 significant digits) · Norm (Norm 1◆:
  exponent form for |x| < 10⁻² or ≥ 10¹⁰; Norm 2: for |x| < 10⁻⁹ or ≥ 10¹⁰)
- **Dezimalpräfixe** (engineering symbols k, M, m, µ …): Ein · Aus◆
- **Bruchergebnis**: ab/c (mixed) · d/c◆ (improper)
- **Komplexe Zahlen**: a+bi◆ · r∠θ (also used for the solutions in Gleichung/Funkt)
- **Statistik**: frequency column Ein · Aus◆
- **Tabellenkalk.**
  - Autom. Berechn.: Ein◆ · Aus
  - Zelle anzeigen: Formel◆ · Wert
- **Gleichung/Funkt**: complex solutions Ein◆ · Aus
- **Tabellen**: f(x) · f(x),g(x)◆
- **Period. Darst.** (recurring decimals): Ein◆ · Aus
- **1000er-Trennung**: Ein · Aus◆
- **Mehrzeilengröße** (linear I/O only): Normale Schrift◆ (4 lines) · Kleine Schrift (6 lines)
- **QR Code**: Version 3 · Version 11◆
- **Automat Aus**: 10 Min◆ · 60 Min
- **Kontrast**: ←/→ adjust

*(RESET)* (pp. 2, 9, 16): **Setupdaten** (setup back to defaults, not the contrast) ·
**Speicher** (Ans, M and the variables) · **Alle initialis.** (everything). Each asks **Ja**.

### 3.3 Berechnungen (pp. 9–21, 23–24)

OPTN (the manual says these come from OPTN "depending on the mode", pp. 17–18):

- **Hyperbol. Fkt.**: sinh, cosh, tanh, sinh⁻¹, cosh⁻¹, tanh⁻¹
- **Winkeleinheit**: °, r, g (mark a value's unit regardless of the setting)
- **Dezimalpräfixe**: m, µ, n, p, f, k, M, G, T, P, E (p. 13)

On keys, with SHIFT or ALPHA:

- Templates: fraction, mixed fraction, √, ⁿ√, x², x³, x⁻¹, xʸ, 10ˣ, eˣ, log, log_a b, ln, |x| (Abs)
- Calculus: ∫ (Gauss–Kronrod, optional tol), d/dx at a point (central differences), Σ, Π
  (pp. 18–20)
- Trigonometry: sin, cos, tan and inverses, in the set angle unit
- Number functions: x!, nPr, nCr, Ran#, RanInt#, Rnd, RndFix, GCD, LCM, Int, Intg, ÷R (with
  remainder), %, *(FACT)* (prime factors, up to 10 digits), recurring-decimal input
- Pol(x; y), Rec(r; θ): results go to x and y
- Sexagesimal °′″ and its conversion
- Engineering notation ENG and ←ENG (p. 13)
- Multi-statements with `:` (p. 13)
- Editing: *(UNDO)*, *(INS)* (wrap the selection into the next function; overwrite mode in
  linear input)
- **CALC** (p. 23): store an expression with variables, ask for their values, evaluate. Also in
  Komplexe Zahlen.
- **SOLVE** (p. 24): Newton's method on an equation; asks for a start value and the other
  variables, lets you pick the unknown; shows the solution and L−R. Berechnungen only.

### 3.4 Komplexe Zahlen (pp. 22–23)

- Keys: *i*, ∠ (polar input)
- OPTN: **Argument/Phase**, **Konjugierte**, **Realteil**, **Imaginärteil**, **▶r∠θ**,
  **▶a+bi**, plus Winkeleinheit as in 3.3
- Abs on its key; CALC works here.

### 3.5 Basis-N (pp. 28–29)

- Number modes on keys: *(DEC)*, *(HEX)*, *(BIN)*, *(OCT)*; A–F typed on the variable keys.
  32 bit, two's complement; fractions and exponents are not allowed.
- OPTN:
  - Prefix for one value: **d**, **h**, **b**, **o**
  - Logic: **and**, **or**, **xor**, **xnor**, **Not**, **Neg**
- A result converts with the same DEC/HEX/BIN/OCT keys.

### 3.6 Matrizen (pp. 31–33)

- OPTN:
  - **Mat. definieren** → MatA · MatB · MatC · MatD → rows (1–4) → columns (1–4) → editor
  - **Mat. bearbeiten** → MatA–D
  - **MatA**, **MatB**, **MatC**, **MatD**, **MatAns** (insert)
  - **Determinante**, **Transponierte**, **Einheitsmatrix** (Identity(n), n = 1–4),
    **Stufenform REF**, **Stufenform RREF**
- On keys: Abs (each element), x⁻¹, x², x³.
- MatAns screen: the result; an operator key continues the calculation with it. STO in an
  editor copies the matrix to MatA–D.

### 3.7 Vektorrechnung (pp. 34–36)

- OPTN:
  - **Vek. definieren** → VctA · VctB · VctC · VctD → dimension 2 or 3 → editor
  - **Vek. bearbeiten** → VctA–D
  - **VctA**–**VctD**, **VctAns**
  - **Skalarprodukt** (•), **Winkel** (Angle(a; b)), **Einheitsvektor** (UnitV)
- × between two vectors is the cross product; Abs is the length.

### 3.8 Statistik (pp. 25–28)

- **Typ auswählen** (shown on entry, and in OPTN): 1 Variable · y=a+bx · y=a+bx+cx² ·
  y=a+b·ln(x) · y=a·e^(bx) · y=a·b^x · y=a·x^b · y=a+b/x
- Statistics editor: columns x / x, Freq / x, y / x, y, Freq (160, 80 or 53 rows).
- OPTN in the editor:
  - **Typ auswählen**
  - **Editor** → Zeile einfügen · Alles löschen
  - **1-Variab-Berech** / **2-Variab-Berech** (all values on one screen)
  - **Regression** (a, b, (c), r on one screen; pairs only)
- AC leaves the editor for the statistics calculation screen; there, OPTN:
  - **Typ auswählen**, **Daten** (back to the editor)
  - **Summierung**: Σx, Σx², Σy, Σy², Σxy, Σx³, Σx²y, Σx⁴
  - **Variable**: n, x̄, ȳ, σ²x, σx, s²x, sx, σ²y, σy, s²y, sy
  - **Minimum/Maximum**: min(x), max(x), min(y), max(y), and for one variable Q1, Med, Q3
  - **Regressionen**: a, b, (c), r, x̂, ŷ (x̂₁, x̂₂ for quadratic)
  - **Normal-Vert.** (one variable): P(, Q(, R(, ▶t
  - **1-Variab-Berech** / **2-Variab-Berech**, **Regression**

### 3.9 Verteilungsfkt. (pp. 38–41)

- **Typ auswählen**: Normal-Dichte · Kumul. Normal-V · Inv. Normal-V. · Binomial-Dichte ·
  Kumul. Binom.-V · Poisson-Dichte · Kumul.Poisson-V
- Binomial and Poisson ask **Liste** (up to 45 values of k) or **Variable** (one k).
- Inputs: Normal-Dichte x, σ, μ · Kumul. Normal-V Untere, Obere, σ, μ · Inv. Normal-V. Fläche,
  σ, μ (tail always left) · Binomial k, n, p · Poisson k, μ.
- OPTN in the list: **Typ auswählen**, **Editor** → Zeile einfügen · Alles löschen.

### 3.10 Tabellenkalk. (pp. 41–45)

- OPTN:
  - **Formel füllen** (formula + cell range), **Wert füllen** (value + cell range)
  - **Zelle bearbeit.**, **Zell-Auswahl** (pick a reference with the cursor), **$** (absolute
    reference)
  - **Ausschn.&Einf.**, **Kopier&Einfüg**
  - **Freier Speicher**, **Alles löschen**, **Neu berechnen**
  - Functions: **Min(**, **Max(**, **Mean(**, **Sum(** over `start:end`
- STO/RECALL move values between cells and A–F, M, x, y.

### 3.11 Tabellen (pp. 33–34)

- Enter f(x) (and g(x) if the setup says so), then **Start** (1), **Ende** (5), **Inkre** (1).
- At most 45 rows for f(x), 30 for f(x), g(x). An x cell can be edited; the row recalculates.

### 3.12 Gleichung/Funkt (pp. 30–31)

- **Gleichungssyst.** → 2 · 3 · 4 unknowns → coefficient editor
- **Polynom-Gleich.** → degree 2 · 3 · 4 → coefficient editor
- = steps through the solutions (√ forms for polynomials); for degree 2 also the x and y of the
  extremum. STO stores the shown solution.

### 3.13 Ungleichungen (pp. 36–37)

- **Polyn.-Ungleich** → degree 2 · 3 · 4 → relation: `>0` · `<0` · `≥0` · `≤0` →
  coefficient editor
- Answers as intervals, or „Alle reell Zahlen“ / „Keine Lösung“.

### 3.14 Berechn prüf (pp. 37–38)

- Type an (in)equality, chain several; OPTN offers `=`, `≠`, `>`, `<`, `≥`, `≤`.
- Answers **Wahr** / **Falsch** (Ans = 1 / 0); = after Wahr starts the next line with the right
  side.

### 3.15 Verhältnisse (p. 37)

- **Typ auswählen**: A/B=X/D · A/B=C/X → editor for A, B, C, D → X.

### 3.16 CONST (pp. 45–46, 53)

47 constants, CODATA 2010.

| Category | Constants |
|---|---|
| Univers. Konst. | h, ħ, c₀, ε₀, μ₀, Z₀, G, l_P, t_P |
| E-magn. Konst. | μ_N, μ_B, e, Φ₀, G₀, K_J, R_K |
| Atom./Nuk.Konst | m_p, m_n, m_e, m_μ, a₀, α, r_e, λ_C, γ_p, λ_Cp, λ_Cn, R∞, μ_p, μ_e, μ_n, μ_μ, m_τ |
| Phys/Chem.Konst | u, F, N_A, k, V_m, R, c₁, c₂, σ |
| Übernomm. Werte | g, atm, R_K-90, K_J-90 |
| Andere | t (0 °C in K) |

### 3.17 CONV (pp. 46, 53–55)

A value followed by a command such as `cm▶in`; NIST SP 811 (2008). Every pair exists in both
directions.

| Category | Pairs (to SI) |
|---|---|
| Länge | in–cm, ft–m, yd–m, mile–km, n mile–m, pc–km, Å–m, fm–m, ch–m, ua–m, l.y.–m, mil–m, fath–m |
| Fläche | acre–m², b–m², a–m², ha–m², ft²–m², in²–cm², mile²–km² |
| Volumen | gal(US)–L, gal(UK)–L, L–m³, bu–L, bbl–L, ton–m³, fl oz(US)–mL, fl oz(UK)–mL, ft³–m³, in³–m³ |
| Winkel | r–rad |
| Masse | oz–g, lb–kg, mton–kg, ton(long)–kg, ton(short)–kg, mcarat–g |
| Zeit | t-yr–s, min–s, h–s, day–s |
| Geschwindigkeit | km/h–m/s, mile/h–m/s, knot–m/s |
| Beschleunigung | Gal–m/s² |
| Drehmoment | N·m–dyn·cm |
| Kraft | dyn–N, lbf–N, kgf–N |
| Druck | atm, mmHg, kgf/cm², bar, dyn/cm², lbf/in², mmH₂O, inHg – Pa; lbf/in²–kPa |
| Energie | kgf·m, cal (15 °C), erg, eV, Btu, cal_IT, cal_th, kW·h – J |
| Leistung | hp–kW |
| Wärmedurchfluss | Btu/min–W |
| Temperatur | °F–°C |
| Spezif. Wärme | cal_th/(g·K)–J/(kg·K) |
| Viskosität | P–Pa·s |
| Kinem. Viskosit | St–m²/s |
| Magnetismus | G–T, Oe–A/m, Mx–Wb, γ–T |
| Lichtstärke | sb–cd/m², ph–lx |
| Radioaktivität | Ci–Bq, rad–Gy, rem–Sv, R–C/kg |

### 3.18 Memory and history (p. 16)

- **Ans** (last result), **PreAns** (the one before, in the Ans key's second function).
- Variables **A, B, C, D, E, F, M, x, y**: STO + letter stores, ALPHA + letter inserts,
  *(RECALL)* shows all nine. **M+**, *(M−)* add to and subtract from M.
- History with ↑/↓; ←/→ on a result edits the last input again. Cleared by a mode change.

## 4. Casio entries mapped to Quantus

*Today* is what Quantus does now. *Decision* is what was agreed on 2026-11-01, entry by entry.
Rows the engine already covers are marked **keep**; **drop** means not on the handheld. The
work the decisions call for, and where it goes (core, desktop, handheld), is in section 8.

| Casio | Today in Quantus | Decision |
|---|---|---|
| Main menu, digit or cursor + OK | MODE tiles, 6 per page (`render.rs`) | keep |
| Berechnungen | Physik (numbers with units and constants) | Physik stays the default mode and tile 1 |
| Komplexe Zahlen | Komplex (`imaginary`) | keep |
| Basis-N | – | own mode, soon: integers, DEC/HEX/BIN/OCT, d/h/b/o, and/or/xor/xnor/Not/Neg |
| Matrizen | only `\det` up to 8×8; Gauss steps in LGS | the full Casio set, exact and with steps: + − ×, inverse, transpose, identity, det, powers, REF/RREF; any variable name instead of MatA–D |
| Vektorrechnung | – | goes into the future Geometrie area, not its own mode |
| Statistik, Verteilungsfkt. | – | one Stochastik module in the engine, two tiles (Statistik, Verteilungen), as in `plans/new-areas-and-themes.md` |
| Tabellenkalk. | – | drop |
| Tabellen | Kurvendiskussion → Wertetabelle (one function, ≤ 201 rows) | its own tile, Wertetabelle, on the analysis code; g(x) as a second column |
| Gleichung/Funkt: Gleichungssyst. | LGS mode, or `;` / `cases`; exact, with steps | LGS tile; OPTN switches between typing equations and the coefficient editor [A\|b] |
| Gleichung/Funkt: Polynom-Gleich. | Algebra `=`: degree ≤ 2 exact, higher numeric on [−100, 100] | no coefficient form: typed in Algebra. The engine gets exact roots for degree 3 and 4 (Cardano, Ferrari) and loses the [−100, 100] limit |
| Vertex after a quadratic | extrema in Kurvendiskussion | Kurvendiskussion is enough |
| Ungleichungen | – | later, low priority |
| Berechn prüf | – | both: Algebra answers wahr/falsch for an equation without unknowns, and a Prüfen tile with the Casio's chaining |
| Verhältnisse | Algebra: `\frac{3}{8} = \frac{x}{12}` | drop, Algebra does it |
| SETUP Eingabe/Ausgabe | 2D input; exact or decimal | only Ergebnis exakt/dezimal; no one-line input |
| SETUP Winkeleinheit | radians; degrees as `30^\circ` | engine setting Grad/Bogenmaß (no gon); degrees by default on the handheld; D/R in the top bar |
| SETUP Zahlenformat | significant digits (handheld 1–15) | Norm 1/2, Fix 0–9, Sci 0–9 as on the Casio; significant digits stay the internal precision |
| SETUP Dezimalpräfixe, ENG | prefixes on units in Physik only | SETUP Dezimalpräfixe (prefixes on plain numbers too) and an ENG / ←ENG key that steps the exponent or prefix by 3 |
| SETUP Bruchergebnis | improper fractions only | setting ab/c / d/c, and S⇔D toggles between them |
| SETUP Komplexe Zahlen, ▶r∠θ, ▶a+bi | a+bi only | setting a+bi / r∠θ, OPTN ▶r∠θ / ▶a+bi for one result, ∠ to type polar values, in the angle unit set |
| SETUP Gleichung/Funkt (complex roots) | Komplex gives complex roots; LGS has its i switch | per mode, no global switch |
| SETUP Statistik (Freq), Tabellen f/g | – | with their modes: Freq column in the Statistik editor; g(x) in Wertetabelle |
| SETUP Period. Darst. | – | recurring decimals in the S⇔D cycle, and a template to type a period |
| SETUP 1000er-Trennung | – | yes: a thin space (the comma is the decimal mark) |
| SETUP Mehrzeilengröße | – | a font size setting, normal or small, for long results and the step list |
| SETUP QR Code | – | a QR code of the result or the steps (LaTeX, for the desktop); no version setting |
| SETUP Automat Aus | deep sleep | keep (firmware) |
| SETUP Kontrast | – | backlight brightness |
| RESET | – | keep (firmware): settings / memory / all |
| Hyperbol. Fkt. | sinh, cosh, tanh | add arsinh, arcosh, artanh (and coth) |
| Winkeleinheit °, r, g in input | `^\circ` | ° and rad mark a value regardless of the setting; no gon |
| ∫, d/dx | numeric ∫ to 30 digits, exact values recognised; d/dx at a point | keep |
| Σ | `\sum`, exact, up to 10⁶ terms | keep |
| Π | – | add `\prod`, like `\sum` |
| x!, nCr, %, Abs | `n!`, `\binom`, `\%`, `\|x\|` | keep |
| nPr, GCD, LCM | – | add |
| FACT | – | add: prime factors as a result form |
| Int, Intg, Rnd, RndFix, ÷R | ⌊⌋, ⌈⌉, mod | add |
| Ran#, RanInt | – | drop |
| Pol, Rec | – | none of their own: x+yi ▶r∠θ in Komplex does the job |
| Sexagesimal °′″ | – | input 2°20′30″, and °′″ as a result form on S⇔D |
| CALC | session variables | a CALC prompt: asks for each free variable of the expression on screen, then evaluates (with the SUBSTITUTED step); the values become session variables |
| SOLVE | Algebra `=` | SOLVE for a chosen variable (F = m·a for a); the others from session variables or asked as in CALC; exact, numeric with an optional start value |
| Komplex OPTN Arg, Conjg, ReP, ImP | `\arg`, `\overline{z}`, `\Re`, `\Im`, `\|z\|` | keep |
| CONST (47, CODATA 2010) | 15, CODATA 2018, Physik only | all 47, CODATA 2018, in the Casio's categories; inserted from CONST they work in every mode |
| CONV (`cm▶in`) | divide by the target unit | an engine operator "▶ unit" that writes the result in that unit; CONV lists target units by the Casio's categories; add the missing units (ft, yd, lb, oz, gal, psi, Btu, hp, °F …) |
| Ans | Ans key | keep |
| PreAns | – | add |
| A–F, M, x, y; STO, RECALL | session variables with any name | STO stores Ans under any name; VARS lists the session's variables to insert, edit or delete |
| M+, M− | – | drop |
| History ↑/↓, replay ←/→ | the same, 50 entries | keep; saved with the session (section 6) |
| UNDO, INS | – | UNDO/REDO, INS (wrap the previous item in the next template), select with SHIFT+←/→, copy and paste |

## 5. What Quantus adds

### 5.1 Modes

| Mode | Engine key | What the handheld offers |
|---|---|---|
| Physik | `physics` | units after numbers, constants, the quantity name (energy, force …), steps SYMBOLIC → SUBSTITUTED → RESULT |
| Algebra | `algebra` | every letter a variable; simplify, factor, cancel fractions with a DOMAIN line, solve, systems |
| Komplex | `imaginary` | *i* is the imaginary unit, results a+bi, complex roots |
| Kurvendiskussion | `analysis` | one f(x): the sections of [analysis-mode.md](analysis-mode.md) plus three tasks: Tangente/Normale at x₀, Fläche (a, b or the outer zeros), Wertetabelle (von, bis, Schritt) |
| LGS | `linear` | input as equations or as the matrix [A\|b] (up to 10×10), exact solution with Gauss or Gauss–Jordan steps, λ, μ for free unknowns |
| Kirchhoff | `kirchhoff` | not on the handheld: it needs a drawn network |
| Elektronik | (hidden) | not on the handheld: circuit editor, hidden in the desktop too |

### 5.2 Settings

From `docs/docs/reference/settings.md`; the hui has all of them (section 7.2).

| Setting | Key | Values |
|---|---|---|
| Signifikante Stellen | `precision` | 1–15 on the handheld (engine: 1–10 000) |
| Ergebnis | `exactRoots` (inverted in `firmware/sim/src/engine.rs`) | exakt · dezimal |
| Sprache | `language` | de · en |
| Rechenweg | `stepDetail` | keiner · kurz · standard · ausführlich · maximal |
| Kürzungen durchstreichen | `cancelMarks` | ein · aus |
| Gleichungssysteme | `elimination` | Gauß · Gauß–Jordan |
| Unbekannte (LGS) | `unknownNames` | indiziert · Buchstaben |
| Imaginäre Einheit i (LGS) | `linearComplex` | ein · aus |
| Binomische Formeln | `binomialForm` | automatisch · ausmultipliziert · als Binom |
| Quadratische Ergänzung | `completeSquare` | ein · aus |
| Nenner | `denominatorForm` | automatisch · ausmultipliziert · faktorisiert |
| Identitäten mit Bedingung | `conditionalIdentities` | mit Bedingung · nur immer gültige |
| Quadratische Gleichungen | `quadraticFormula` | automatisch · pq · abc |
| a und y als Variablen | `aAndYAsVariables` | ein · aus |
| Buchstabenfolge als Produkt | `letterProducts` | ein · aus |
| t, u und in als Einheiten | `extraUnitLetters` | ein · aus |

### 5.3 Input the Casio has no menu for

- **Units** (Physik; elsewhere in `\mathrm{}`): the SI units, l, eV, cal, Wh, bar, min, h, d, wk,
  mo, yr, atm, Torr, ha, Å, mph, ly, AU, Da, % (`units-and-constants.md`). A picker by quantity
  (Länge, Masse, Zeit, Strom, Temperatur, Kraft, Druck, Energie, Leistung, Ladung, Spannung,
  Widerstand, Kapazität …) is easier than typing them with ALPHA.
- **Prefixes**: all 24, q to Q; the Casio has 11. µ needs its own entry (`u` is the dalton).
- **Limits**: `\lim_{x\to a}`, with a⁺, a⁻ and ∞.
- **Number functions**: ⌊x⌋, ⌈x⌉, max(a; b), min(a; b), mod, log_b, cot, sec, csc.
- **Variable names**: a subscript key exists (`Key::Subscript`); Greek letters need a menu
  (α β γ δ ε θ λ μ ρ σ τ φ ω, with Δ and Ω).
- **Systems**: `;` between equations (`Key::Semicolon`), and `cases`.
- **Infinity** as an integral bound and limit target.

### 5.4 CONST and CONV in Quantus

- **CONST**: all 47 of the Casio's constants with CODATA 2018 values, in its six categories.
  Quantus has 15 of them today: Univers. c₀ (`c_0`), h (`h_P`), ħ, ε₀, μ₀, G (`G_N`); E-magn.
  e (`e_0`); Atom./Nuk. m_e, m_p, m_n; Phys/Chem. N_A, k (`k_B`), R (`R_m`), σ (`\sigma_SB`);
  Übernomm. g (`g_n`). The menu inserts a constant by its name, so it works outside Physik too.
- **CONV**: a "▶ unit" operator in the engine, `5\,\mathrm{cm} ▶ \mathrm{in}`, that writes the
  result in the target unit (compatible dimension, else an error). The menu lists the target
  units in the Casio's 21 categories (3.17), only those Quantus knows.

### 5.5 On the result screen

- The step list (Rechenweg), scrollable, at the level of `stepDetail`.
- The DOMAIN line for fractions in Algebra.
- The quantity name in Physik.
- Kurvendiskussion: one page per section, ←/→ between sections.
- LGS: the Gauss steps row by row.
- S⇔D cycles through the forms a result has: exact, d/c or ab/c, recurring decimal, decimal,
  and °′″ for angles. Prime factors (FACT), ENG and the polar form come from OPTN or their keys.
- A QR code of the result or the steps.

## 6. Sessions

Sessions are a feature of `quantus-core`, used by the desktop and the handheld alike. The core
already has them in memory (`crates/quantus-core/src/session.rs`: `Session` with the variables,
`SessionStore` by id); the history is kept twice today, in the desktop frontend and in
`firmware/ui/src/app.rs`. Both move into the core session.

A session holds:

- the session variables,
- the history,
- what the modes hold: the LGS matrix, the Kurvendiskussion function, the Wertetabelle range,
  the Statistik lists, the matrices,
- its name, and when it was created and last used.

The mode and the settings are not part of a session; they stay global.

- **Core**: the session type above, serialised to and from JSON (serde, as `Settings` already
  is), with a format version. The core does not touch files; it hands the JSON to the shell
  and takes it back.
- **Desktop**: stores one file per session in the app's config directory, next to
  `settings.json`; a session menu (open, new, rename, duplicate, delete) and opening the last
  session on start. Tracked in `plans/plans.md` §5.
- **Handheld**: stores the same files on the SD card, after every calculation and before deep
  sleep, so a power loss loses at most the last input; the *Sitzungen* menu from MODE or
  SHIFT + a key; the last session reopens on wake-up.
- **Exchange**: the files are the same on both, so a session moves over the card or USB.
- **Names**: a new session starts as "Sitzung 2026-11-01 14:30" and can be renamed.

Deep sleep does not keep the RAM: on the ESP32-P4 the main SRAM and the PSRAM are off, and only
the small low-power memory survives. That is why the handheld writes sessions to the SD card.
How long deep sleep lasts depends less on the chip (tens of µA) than on the rest of the
board: the dev board's regulator, USB bridge and LED and the display module's regulator and
shift registers can draw 1–10 mA unless they can be switched off, and the backlight needs a
switch. At 5 mA a 2000 mAh LiPo lasts about 2½ weeks; at 50 µA, years. These are estimates,
to be measured; they belong to the power question in
[calculator-hardware.md](calculator-hardware.md) §6.

## 7. The handheld tree

**[C]** from the Casio, **[Q]** Quantus only, **[eng]** needs work in `quantus-core` first
(section 8), which the desktop gets too, **[fw]** firmware only. Six entries per page, as the
mode menu draws them.

Status in the hui (`firmware/ui`), as implemented: **✓** works (the engine gets the setting
through the `Backend`, or the hui does it itself), **◌** UI only (the value is kept, or the
entry says *Noch nicht verfügbar*) until the engine or the firmware can do it. The hui's texts
are German and English (`firmware/ui/src/text.rs`), German by default; *Sprache* switches them.

### 7.1 MODE (main menu)

In the hui (`Mode` in `firmware/ui/src/app.rs`): all twelve tiles, ←/→ through all tiles, ↑/↓
by a row and across the pages, a digit picks 1–9 and 10–12 need the cursor (section 9). A long
name breaks at a soft hyphen, and goes small when a word is still too wide. A mode without the
engine (◌) can be chosen and shows its name, and EXE answers *Noch nicht verfügbar*.

- Page 1
  1. Physik [C Berechnungen + Q], the default ✓
  2. Algebra [Q] ✓
  3. Komplex [C] ✓
  4. Kurvendiskussion [Q] ✓ (plain inputs; no screen for the sections yet)
  5. LGS [C Gleichungssyst. + Q] ✓ (typed systems; no matrix editor yet)
  6. Wertetabelle [C Tabellen, on the analysis code] ◌
- Page 2
  7. Matrizen [C, eng] ◌
  8. Basis-N [C, eng] ◌
  9. Statistik [C, eng] ◌
  10. Verteilungen [C, eng] ◌
  11. Prüfen [C Berechn prüf, eng] ◌
  12. Sitzungen [Q, fw] ◌: a page Neu, Öffnen, Umbenennen, Duplizieren, Löschen, each *Noch nicht
      verfügbar* until the core has sessions (section 6)
- Later: Ungleichungen [C, eng], Geometrie with the vectors [eng]
  ([plans/new-areas-and-themes.md](../plans/new-areas-and-themes.md)).
- Not on the handheld: Tabellenkalk., Verhältnisse (typed in Algebra), Kirchhoff and
  Elektronik (they need a drawn circuit).

### 7.2 SETUP (SHIFT + MODE)

**Every entry opens a submenu** in which its value is changed: a list of its values (↑/↓ or the
digit, OK), or a number typed with the digit keys (DEL deletes, − for a sign, OK stores it if it
is in range, else the range shown below turns red). ←/→ do not step values. A digit picks the
numbered entry of a page, ↑/↓ move and scroll (six lines per page), OK/EXE/→ open, ← or DEL go
back one level, AC closes SETUP. The tree is `firmware/ui/src/tree.rs`, the machinery
`menu.rs`, the values `settings.rs`; the simulator's engine (`firmware/sim/src/engine.rs`)
maps every setting the core has.

- **Allgemein**: Sprache ✓ (the hui's texts and the engine's) · Winkeleinheit Grad/Bogenmaß
  ✓ (the simulator passes it on) · Signifikante Stellen ✓ (typed, 1–15) · Zahlenformat Norm 1/Norm 2/Fix n/Sci n [eng] ◌
  (Fix and Sci ask for n, 0–9) · Dezimalpräfixe [eng] ◌ · 1000er-Trennung [eng] ◌
- **Ergebnis**: exakt/dezimal ✓ · Brüche d/c, ab/c [eng] ◌ · Periodisch [eng] ◌ · Komplexe
  Zahlen a+bi/Polar (r, θ) [eng] ◌ (the fonts have no ∠) · Quadratische Gleichungen Auto/pq/abc ✓
- **Rechenweg**: Rechenweg (5 levels) ✓ · Kürzungen durchstreichen ✓ · Gleichungssysteme
  Gauß/Gauß-Jordan ✓ (the hui has no step list yet; the engine gets them)
- **Algebra**: Binomformeln ✓ · Quadratische Ergänzung ✓ · Nenner ✓ · Identitäten mit Bedingung ✓
- **Eingabe**: a und y als Variablen ✓ · Buchstaben als Produkt ✓ · t, u, in als Einheiten ✓ ·
  Unbekannte (LGS) ✓
- **Gerät** [fw]:
  - Helligkeit ◌ (typed, 1–10)
  - Schriftgröße normal/klein ◌
  - Hintergrundbild ✓ in the hui and the simulator (section 7.6): Bild (Keins, then the BMP
    files in `/wallpapers/` on the SD card, with a preview beside the list) · Anzeigen
    Hauptmenü/Menüs/Überall
  - Automatisch aus 10/60 min ◌
  - Zurücksetzen → Einstellungen / Speicher / Alles, each with *Ja* ✓ (the settings to their
    defaults; the history, Ans and the input cleared; both)
- **Verbindungen** [fw], section 7.5:
  - **Prüfungsmodus**: ein · aus ✓ in the hui: on switches WLAN and Bluetooth off, and their
    *Ein* then says *Aus im Prüfungsmodus*; the top bar shows it. No radio is touched yet.
  - **WLAN**: ein · aus ◌ · Netzwerke (scan, connect, password) ◌ · Gespeicherte Netzwerke
    (forget) ◌ · Status (network, IP, signal) ◌
  - **Rechnen im Netz**: aus · ab 500 ms ◌ (hand long calculations to a Quantus engine on the
    desktop or a server) · Adresse ◌ (needs text input)
  - **Sitzungen abgleichen**: with the desktop app over WLAN or Bluetooth · jetzt ◌ ·
    automatisch ◌
  - **Uhrzeit**: Quelle Netzwerk (NTP) / von Hand ◌ · Stunde, Minute ◌ (typed) · Zeitzone ◌
    (typed, −12 to +14)
  - **Firmware-Update**: installierte Version ✓ (the crate's version) · suchen ◌ · installieren ◌
    (the latest GitHub release, over WLAN); updating from the desktop app works too, section 7.5
  - **Bluetooth**: ein · aus ◌ · Koppeln (desktop app, phone, keyboard) ◌ · Gekoppelte Geräte
    (forget) ◌ · Sichtbar ein/aus ◌

### 7.3 OPTN per mode

In the hui: the OPTN key (`Key::Optn`; F4 or Ctrl+N in the simulator and over serial) opens the
mode's page in the same menus as SETUP (`tree::optn`); OPTN or AC closes it. An entry inserts
its LaTeX at the cursor (✓, through `latex::parse`; into the brackets for `sinh(`, between the
bars for |x|), or says *Noch nicht verfügbar* (◌). The fonts lack →, ⌊⌋, ⌈⌉ and µ: lim shows
no arrow, floor and ceiling show as their names (`lfloor`), micro is inserted as μ (`\mu`).

Shared groups, used by several modes:

- **Analysis**: ∫ ✓ (`\int_{}^{}`), d/dx ✓, Σ ✓ (`\sum`, now drawn as ∑), Π [eng] ◌, lim ✓
- **Zahl**: |x| ✓, n! ✓, nCr ◌ (the engine has `\binom`, the editor no template), nPr [eng] ◌,
  ggT, kgV [eng] ◌, Int, Intg, Rnd, RndFix, ÷R [eng] ◌, mod ✓ (`\bmod`), ⌊⌋, ⌈⌉ ✓, max, min ✓
  (`\max(;)`), % ✓
- **Hyperbol. Fkt.**: sinh, cosh, tanh ✓, arsinh, arcosh, artanh [eng] ◌
- **Winkel**: ° ✓, rad ✓, °′″ [eng] ◌
- **Griechisch**: α β γ δ ε θ λ μ ρ σ τ φ ω Δ Ω ✓
- **Werkzeuge**: CALC [eng] ◌, SOLVE [eng] ◌, ▶ Einheit [eng] ◌

Per mode:

- **Physik**: Einheiten (by quantity) [Q] ✓ (the units of section 5.3 in 15 quantities,
  inserted as letters) · Präfixe [Q] ✓ (all 24, as letters, μ as `\mu`) · Analysis · Zahl ·
  Winkel · Hyperbol. Fkt. · Werkzeuge
- **Algebra**: Gleichungssystem (`;` ✓, `cases` ◌, no template) [Q] · Griechisch · Analysis ·
  Zahl · Hyperbol. Fkt. · Werkzeuge
- **Komplex**: Argument, Realteil, Imaginärteil, Betrag ✓, Konjugierte ◌ (no `\overline`
  template) [C] · ▶r∠θ, ▶a+bi, ∠ [eng] ◌ · Analysis · Zahl
- **Kurvendiskussion**: the sections · Tangente/Normale · Fläche · Wertetabelle [Q], all ◌ until
  the mode has its screens
- **Wertetabelle**: f(x) · g(x) · von, bis, Schritt [C] ◌
- **LGS**: Eingabe Gleichungen / Matrix [A|b] ◌ (kept) · Größe as Zeilen and Spalten, typed,
  up to 10 ◌ (kept) · Imaginäre Einheit i ✓ (`linearComplex`) · Alles löschen ✓ [Q]
- **Matrizen**: Definieren, Bearbeiten · the matrix variables · Ans · Determinante,
  Transponierte, Inverse, Einheitsmatrix, REF, RREF [C, eng] ◌
- **Basis-N**: DEC, HEX, BIN, OCT · d, h, b, o · and, or, xor, xnor, Not, Neg [C, eng] ◌
- **Statistik**: Typ (1 Variable and the 7 regressions) · Editor: Zeile einfügen, Alles löschen ·
  Summen, Kennwerte, Min/Max/Quartile, Regression [C, eng] ◌
- **Verteilungen**: Normal-Dichte, Kumul. Normal-V, Inv. Normal-V, Binomial-Dichte, Kumul.
  Binom.-V, Poisson-Dichte, Kumul. Poisson-V · Liste/Variable [C, eng] ◌
- **Prüfen**: =, ≠, <, >, ≤, ≥ [C, eng] ✓ as inserts; the mode itself waits for the engine

### 7.4 Keys

| Function | Handheld | Status in `key.rs` |
|---|---|---|
| Main menu | MODE | exists |
| SETUP | SHIFT + MODE | exists |
| OPTN | OPTN | exists (`Optn`; F4, Ctrl+N) |
| CONST, CONV | own key or SHIFT + a key | missing |
| S⇔D | S⇔D | exists (`SwapForm`) |
| ENG, ←ENG | a key and SHIFT + it | missing |
| STO, VARS | SHIFT + a key, and a key | missing |
| CALC, SOLVE | a key and SHIFT + it, or OPTN → Werkzeuge | missing |
| Sitzungen | SHIFT + a key, or the MODE tile | missing |
| PreAns | SHIFT + Ans | missing |
| ∠ | SHIFT + a key in Komplex | missing |
| UNDO/REDO, INS | SHIFT + DEL, SHIFT + another key | missing |
| Select, copy, paste | SHIFT + ←/→, then a copy and a paste key | missing |
| ∫, d/dx, Σ | own keys or OPTN → Analysis | missing |
| x⁻¹, x³, ⁿ√, 10ˣ, eˣ | SHIFT layers | partly (`Root` with SHIFT is ⁿ√) |
| ALPHA letters | ALPHA + key | `Alpha` exists, legend open |

### 7.5 Connections

The ESP32-P4 has no radio; the ESP32-C6 on the board provides both, over ESP-Hosted
([calculator-hardware.md](calculator-hardware.md) §3).

- **WLAN** is for: the remote engine (calculations over about 500 ms, the goal in
  calculator-hardware.md §1), session sync with the desktop app, firmware updates over the air,
  and the clock (NTP; there is no RTC battery, and sessions carry timestamps).
- **Bluetooth** (BLE) is for: a link to the desktop app (sessions and results, and the handheld as
  a keypad for the app), a phone companion (share results and sessions, set up WLAN), and an
  external keyboard.
- **Prüfungsmodus** turns both off and keeps them off, for exams: a switch in SETUP and the icon
  in the top bar, nothing more. Neither radio is on unless it is switched on; the defaults are
  off.
- **Bluetooth over ESP-Hosted**: the board advertises Bluetooth 5, so the P4 can use the C6's
  BLE; how ESP-Hosted exposes it is a firmware detail.
- **Firmware updates**: the images are assets of the GitHub releases. The desktop app has an
  updater for the handheld: it takes the image from the releases page and uploads it to the
  handheld directly (USB, or the WLAN/Bluetooth link). The handheld can also fetch the same
  release itself over WLAN. The release workflow ([RELEASING.md](../RELEASING.md)) then has to
  attach the firmware image.
- **Top bar**: icons for WLAN, Bluetooth and the exam mode, next to the battery. *Implemented*
  in the hui:
  - a Wi-Fi fan and the Bluetooth rune while on, a *PRÜFUNG* tag in the accent;
  - D/R for the angle unit (section 8, item 1) next to the digits and *Exakt*/*Dez*;
  - while USB power is in, a bolt inside the battery and a green fill (`App::charging`, set by
    the board);
  - while a calculation runs elsewhere (the remote engine), a ring of dots turning after the
    mode's name (`App::busy`, set by the client sending the request).

### 7.6 Colours

The hui uses the desktop's purple palette everywhere (`desktop/src/styles.css`): the top bar,
the mode tiles, the menu highlight and the cursor in lavender `#9186e8` (`--cyan`) and the accent
`#d8a9ff` (`--amber`), their dim forms `#423a63` and `#4e3c63` for fills, on the background
and surfaces the hui already shares with the desktop (`firmware/ui/src/render.rs`). The colours per
mode in `firmware/ui/src/app.rs` (green, brown, teal, slate) go; the mode shows by its name in
the top bar and on its tile.

*Implemented* in the hui: `render::colors::{CYAN, AMBER, CYAN_DIM, AMBER_DIM}`; the bar in
`CYAN_DIM`, the caret in `CYAN`. Each mode tile has its own muted colour (`colors::TILES`), the
chosen tile a lighter form of it with a `TEXT` outline; a result shows in a soft purple `#bfa0f2`.

**Wallpaper** (`firmware/ui/src/wallpaper.rs`): a picture of the screen's size, 480×320, from
`/wallpapers/` on the SD card, as a BMP (`tinybmp` reads every BMP colour depth). The backend lists
and reads the files; the hui decodes the picture once into two copies, one at full strength and one
mixed 70 % into the background. The full copy lies behind the mode tiles. With *Anzeigen* on
*Menüs* or *Überall*, the dimmed copy also lies behind SETUP/OPTN or behind the calculation as
well, where text stands on it. Over a picture, the tiles and menu lines are glass (`Shape::Glass`,
200 of 256 of their colour), so it shows through. The renderer starts each band from the
picture's rows instead of the background colour; a still picture changes no band, so it costs
no transfers. The two copies take 600 KB, which goes in the P4's PSRAM.

## 8. Where the work goes

The rule: what is maths, data or state goes into `quantus-core`, so the desktop and the handheld
get it alike; the shells add only their UI and their storage. The desktop then gets every new
setting in its settings dialog (`docs/docs/reference/settings.md`, `en.ts` and `de.ts`) and every
new function in its syntax, not just the handheld.

Roughly in the order agreed; each is an item in `plans/plans.md` §5.

| # | Work | Core | Desktop | Handheld |
|---|---|---|---|---|
| 1 | Angle unit Grad/Bogenmaß; ° and rad markers; °′″ input and result form | setting `angleUnit`, parser, result form | settings dialog, top bar | SETUP, D/R in the top bar |
| 2 | Number format Norm/Fix/Sci; Dezimalpräfixe on plain numbers; ENG steps; digit grouping | settings and the number formatter | settings dialog; ENG on a result | SETUP, ENG key |
| 3 | Result forms: mixed fractions, recurring decimals (and their input), prime factors, complex r∠θ with ∠ input | each result carries its alternative forms; settings `fractionForm`, `complexForm`, `recurring` | a form switch on the result | S⇔D cycle, OPTN |
| 4 | Basis-N | a mode `baseN`: integers, bases, bitwise operations | a mode | a tile |
| 5 | Matrices | matrix values and operations, exact, with steps (reusing the LGS Gauss code) | matrix input and output | Matrizen tile and editor |
| 6 | Stochastik | lists, statistics, regressions, distributions | a mode with a data editor | Statistik and Verteilungen tiles |
| 7 | Functions: arsinh, arcosh, artanh, coth, `\prod`, nPr, ggT, kgV, Int, Intg, Rnd, RndFix, ÷R | parser and evaluator | syntax reference | OPTN groups |
| 8 | Constants and units: the 32 missing constants (CODATA 2018), the ▶ unit operator, the missing imperial units | constant and unit tables, the operator | a constants and units picker | CONST, CONV |
| 9 | Solving: exact cubics and quartics, no [−100, 100] limit; SOLVE for a chosen variable; CALC (free variables, substitute); true/false for equations without unknowns | solver and API calls: free variables of an expression, solve for a named variable | SOLVE and CALC in the input's context menu; Prüfen mode | SOLVE, CALC, Prüfen tile |
| 10 | Sessions with history and mode inputs, as JSON (section 6) | `session.rs`: history, mode inputs, names, serde | files in the config directory, session menu | files on the SD card, Sitzungen menu |
| 11 | PreAns, STO, VARS | Ans and PreAns in the session | already has the variables panel | keys and VARS screen |
| 12 | Remote engine and the link to the desktop: LaTeX in, LaTeX out, sessions | the request and response format (the JSON of `api.rs`), the session files | a listener the handheld can reach over WLAN and Bluetooth, pairing | client, Verbindungen page |
| 13 | Firmware updates | – | an updater for the handheld: the image from the GitHub releases, uploaded over USB or the link | receive and install an image (from the desktop or over WLAN); the release workflow attaches it |
| 14 | Later: inequalities; vectors with Geometrie | solver, Geometrie module | – | – |

Firmware only, because they concern the device: UNDO/REDO, INS, selection, copy and paste in the
editor (the desktop's MathLive has them), brightness, font size, auto-off, RESET, the QR code,
the purple palette (7.6), the WLAN and Bluetooth stacks, OTA updates, NTP, the phone companion
and the external keyboard, and the exam mode.

## 9. Open questions

- **Keys**: which of OPTN, CONST, CONV, ENG, STO, VARS, CALC, SOLVE, ∫ and d/dx get their own
  key among the 53, and which go through OPTN.
- **More than nine entries** in a menu: the Casio picks with digits; past 9 the handheld needs the
  cursor, or letters.
- **Cardano and Ferrari** give long nested roots: show them only when they simplify, or always
  with the decimal next to them?
- **Session format**: how mode inputs are stored, and how the format version is migrated.
- **Desktop settings dialog**: it grows by about six settings (angle unit, number format,
  prefixes, fraction form, complex form, recurring decimals); maybe a new group for them.
- **Power**: measure deep sleep on the real board before choosing the battery.
- **Phone companion**: an app, or a web page the handheld serves? Not decided yet, on purpose.
