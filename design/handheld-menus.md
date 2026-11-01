# Handheld menus

The menu structure for the handheld (`firmware/ui`), modelled on the Casio fx-991DE X. Section 3
replicates the Casio's menus; sections 4 and 5 map them to Quantus and add what Quantus can do
that the Casio cannot; section 6 is the merged tree the firmware should implement.

Today the firmware has a MODE tile menu with five modes and a two-row setup (SHIFT+MODE) for
significant digits and exact or decimal results (`firmware/ui/src/app.rs`). The keypad is
"modelled on the Casio fx-DE line, not a copy" ([calculator-hardware.md](calculator-hardware.md)
§4) and its legend is not fixed yet ([plans.md](../plans/plans.md) §4).

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

**Keep**: the engine does it. **Adapt**: the engine does it differently. **Gap**: the engine
cannot do it yet. **Drop**: not wanted on the handheld.

| Casio | Quantus | Status |
|---|---|---|
| Main menu, digit or cursor + OK | MODE tiles, 6 per page (`render.rs`) | keep |
| Berechnungen | Physik (the default mode: numbers with units and constants) | adapt |
| Komplexe Zahlen | Komplex (`imaginary`) | keep |
| Basis-N | – | gap |
| Matrizen | only `\det` up to 8×8 (`syntax.md`); Gauss steps in LGS | gap (operations), keep (det) |
| Vektorrechnung | – (Geometrie with vectors: `plans/new-areas-and-themes.md`) | gap |
| Statistik, Verteilungsfkt. | – (Stochastik: `plans/new-areas-and-themes.md`) | gap |
| Tabellenkalk. | – | drop |
| Tabellen | Kurvendiskussion → Wertetabelle (one function, ≤ 201 rows) | adapt |
| Gleichung/Funkt: Gleichungssyst. | LGS mode, or `;` / `cases` in any mode; exact, with steps | adapt |
| Gleichung/Funkt: Polynom-Gleich. | Algebra: `=` with one unknown; degree ≤ 2 exact, higher numeric on [−100, 100] | adapt |
| Ungleichungen | – | gap |
| Berechn prüf | – (Algebra simplifies both sides, but gives no true/false) | gap |
| Verhältnisse | Algebra: type `\frac{3}{8} = \frac{x}{12}` | adapt (no mode) |
| SETUP Eingabe/Ausgabe Math/Lin. | input is always 2D; output exact or decimal | adapt |
| SETUP Winkeleinheit | radians; degrees written `30^\circ`; no setting | gap |
| SETUP Zahlenformat Fix/Sci/Norm | significant digits (`precision`, firmware 1–15) | adapt, Fix/Sci gap |
| SETUP Dezimalpräfixe, ENG | Physik writes units with prefixes; no ENG on plain numbers | gap |
| SETUP Bruchergebnis ab/c, d/c | improper fractions only | gap (mixed) |
| SETUP Komplexe Zahlen r∠θ | a+bi only (`limits.md`) | gap |
| SETUP Statistik, Tabellenkalk. | follow their modes | drop / gap |
| SETUP Gleichung/Funkt complex | `linearComplex` in LGS; Komplex mode gives complex roots | adapt |
| SETUP Tabellen f(x), g(x) | – | drop |
| SETUP Period. Darst. | – | gap |
| SETUP 1000er-Trennung | – | gap (display only) |
| SETUP Mehrzeilengröße | – | drop (no linear mode) |
| SETUP QR Code | – | drop |
| SETUP Automat Aus | deep sleep ([calculator-hardware.md](calculator-hardware.md) §5) | keep (firmware) |
| SETUP Kontrast | backlight brightness instead | adapt (firmware) |
| RESET | settings / memory / all | keep (firmware) |
| Hyperbol. Fkt. | sinh, cosh, tanh; no inverses | keep, gap (inverses) |
| Winkeleinheit °, r, g | `^\circ`; no r, g | keep (°), gap |
| ∫, d/dx | `\int_a^b` numerical to at most 30 digits, exact values recognised; d/dx at a point | keep |
| Σ, Π | `\sum` exact up to 10⁶ terms; no `\prod` | keep, gap (Π) |
| lim | – on the Casio | Quantus extra, see 5.3 |
| x!, nCr | `n!`, `\binom{n}{k}` | keep |
| nPr, GCD, LCM, Ran#, RanInt, Rnd, RndFix, Int, Intg, ÷R, FACT | `\bmod`, ⌊⌋, ⌈⌉, max, min exist; the rest do not | gap |
| % | `20\%` | keep |
| Pol, Rec | – | gap |
| Sexagesimal °′″ | – | gap |
| CALC | session variables `x_1 = …`, then evaluate | adapt |
| SOLVE | Algebra `=` solves exactly where it can | adapt |
| Komplex OPTN Arg, Conjg, ReP, ImP | `\arg`, `\overline{z}`, `\Re`, `\Im`, `\|z\|` | keep |
| Komplex ▶r∠θ, ▶a+bi | – | gap |
| CONST (47, CODATA 2010) | 15 constants, CODATA 2018, Physik only | adapt (see 5.4) |
| CONV (`cm▶in`) | units are part of the value; convert by dividing by the target unit | adapt (see 5.4) |
| Ans | Ans key | keep |
| PreAns | – | gap (history has the value) |
| A–F, M, x, y; STO, RECALL | any indexed name (`m_1`, `v_{max}`) as a session variable | adapt |
| M+, M− | – | drop |
| History ↑/↓, replay ←/→ | the same, 50 entries (firmware) | keep |
| UNDO, INS | – | gap (firmware editor) |

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

From `docs/docs/reference/settings.md`; the firmware has only the first two so far.

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

- **CONST**: the 15 Quantus constants, in Casio's categories so the menu feels the same.
  Univers.: c₀ (`c_0`), h (`h_P`), ħ, ε₀, μ₀, G (`G_N`). E-magn.: e (`e_0`).
  Atom./Nuk.: m_e, m_p, m_n. Phys/Chem.: N_A, k (`k_B`), R (`R_m`), σ (`\sigma_SB`).
  Übernomm.: g (`g_n`). Casio's other 32 are a gap; the most asked for (u, F, Z₀, a₀, α, R∞,
  μ_B, Φ₀) first.
- **CONV**: no command. The menu inserts `/\mathrm{unit}` so the result reads in that unit, by
  the same categories as 3.17 where Quantus knows the unit (in, mile, h, km/h, mph, atm, bar,
  Torr, eV, cal, kWh, °C, ly, AU, Å …). A real "▶ unit" command in the engine would be better.

### 5.5 On the result screen

- The step list (Rechenweg), scrollable, at the level of `stepDetail`.
- The DOMAIN line for fractions in Algebra.
- The quantity name in Physik.
- Kurvendiskussion: one page per section, ←/→ between sections.
- LGS: the Gauss steps row by row.
- S⇔D as today; FORMAT (Casio CW) is not needed while the only forms are exact and decimal.

## 6. The handheld tree

**[C]** from the Casio, **[Q]** Quantus only, **[gap]** needs the engine first, **[fw]**
firmware only. Six entries per page, as the mode menu draws them.

### 6.1 MODE (main menu)

- Page 1
  1. Physik [C Berechnungen + Q]
  2. Algebra [Q]
  3. Komplex [C]
  4. Kurvendiskussion [Q, replaces Tabellen]
  5. LGS [C Gleichungssyst. + Q]
  6. Gleichungen [C Gleichung/Funkt: Polynom-Gleich., via Algebra]
- Page 2, each [gap] until the engine has it
  7. Matrizen
  8. Vektoren
  9. Statistik
  10. Verteilungen
  11. Ungleichungen
  12. Basis-N
- Later: Berechn prüf [gap], Geometrie and Formelsammlung ([plans/new-areas-and-themes.md](../plans/new-areas-and-themes.md)).
  Not on the handheld: Tabellenkalk., Verhältnisse (Algebra does it), Kirchhoff, Elektronik.

### 6.2 SETUP (SHIFT + MODE)

- **Allgemein**: Sprache · Signifikante Stellen · Winkeleinheit [gap] · Zahlenformat Norm/Fix/Sci
  [gap] · Dezimalpräfixe [gap] · 1000er-Trennung [gap]
- **Ergebnis**: exakt/dezimal · Bruch d/c, ab/c [gap] · Periodisch [gap] · Komplex a+bi/r∠θ
  [gap] · Quadratische Gleichungen pq/abc
- **Rechenweg**: Rechenweg (5 levels) · Kürzungen durchstreichen · Gleichungssysteme Gauß/Gauß–Jordan
- **Algebra**: Binomische Formeln · Quadratische Ergänzung · Nenner · Identitäten mit Bedingung
- **Eingabe**: a und y als Variablen · Buchstabenfolge als Produkt · t, u und in als Einheiten ·
  Unbekannte (LGS) · Imaginäre Einheit i (LGS)
- **Gerät** [fw]: Helligkeit · Automatisch aus 10/60 min · Zurücksetzen → Einstellungen /
  Speicher / Alles (each with Ja)

### 6.3 OPTN per mode

- **Physik**: Einheiten (by quantity) [Q] · Präfixe [Q] · Konstanten (= CONST) · Winkel ° [C] ·
  Hyperbol. Fkt. [C] · Analysis: ∫, d/dx, Σ, lim [C + Q] · Zahl: |x|, n!, binom, mod, ⌊⌋, ⌈⌉,
  max, min, % [C + Q]
- **Algebra**: Gleichungssystem (`;`, `cases`) [Q] · Griechisch [Q] · Analysis · Zahl ·
  Hyperbol. Fkt.
- **Komplex**: Argument, Konjugierte, Realteil, Imaginärteil, Betrag [C] · ▶r∠θ, ▶a+bi [gap] ·
  Analysis · Zahl
- **Kurvendiskussion**: Kurvendiskussion (the sections) · Tangente/Normale · Fläche ·
  Wertetabelle (von, bis, Schritt) [Q]; the result pages as in 5.5
- **LGS**: Eingabe Gleichungen / Matrix [A|b] · Größe (m × n, up to 10) · Alles löschen [Q]
- **Gleichungen**: Polynom Grad 2 · 3 · 4 → coefficient editor [C]; the solutions go to the
  Algebra solver, so the result is exact with steps

### 6.4 Keys

| Casio | Handheld | Status in `key.rs` |
|---|---|---|
| MENU | MODE | exists |
| SHIFT + MENU (SETUP) | SHIFT + MODE | exists |
| OPTN | OPTN | missing |
| CONST | SHIFT + a key | missing |
| CONV | SHIFT + a key | missing |
| S⇔D | S⇔D | exists (`SwapForm`) |
| STO, RECALL | STO (SHIFT + a key) for `name = Ans`, VARS to list the session variables | missing |
| CALC, SOLVE | not needed (5.4, section 4) | – |
| ∫, d/dx, Σ | own keys or OPTN → Analysis | missing |
| x⁻¹, x³, ⁿ√, 10ˣ, eˣ | SHIFT layers | partly (`Root` with SHIFT is ⁿ√) |
| ALPHA letters | ALPHA + key | `Alpha` exists, legend open |

## 7. Open questions

- **Angle unit**: add a degree/radian setting to the engine, or keep radians and offer ° only in
  OPTN? The Casio defaults to degrees, which school users will expect.
- **Number format**: significant digits only, or Fix/Sci/Norm as the Casio has them?
- **Order of the gaps**: Matrizen and Statistik are the ones most missed against the Casio;
  Basis-N and Ungleichungen are small.
- **Keys**: which of OPTN, CONST, CONV, STO, ∫ and d/dx get their own key in the 53, and which go
  through menus.
- **More than nine entries** in a menu: the Casio picks with digits; past 9 the handheld needs the
  cursor, or letters.
