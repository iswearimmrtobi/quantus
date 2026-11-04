# Changelog

All notable changes to Quantus. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- The desktop app shows the Casio features of the engine:
  - Settings for the angle unit, the number format (Norm 1/2, Fix, Sci), decimal prefixes, digit
    grouping, fractions, recurring decimals and complex numbers.
  - Buttons under the result switch between its forms (fraction, mixed, recurring, decimal,
    exact, prime factors, °′″, a + bi and r∠θ, the other bases), also with Ctrl+D; ENG and ←ENG
    (Ctrl+E, Ctrl+Shift+E) write it in engineering notation.
  - The modes **Base-N** (Basis-N), with a DEC/HEX/BIN/OCT switch and keys for the logic and the
    prefixes, and **Verify** (Prüfen), with keys for the relations.
  - A **Library** window (Ctrl+K) with the 47 constants and the units to convert to, by category.
  - A **quick search** (Ctrl+F) for constants and units: type part of the name (`steff`), Enter
    puts the first hit in at the cursor. It finds by symbol, by name in the set language or in
    English, and forgives a typo or two.
  - A **Writeup** window (Ctrl+G) writes the session up as a physics solution: given values
    and wanted quantities side by side, then for each quantity the law it follows ("Nach dem
    Ohm'schen Gesetz gilt …"), the rearrangement, the values put in, the result and a unit
    check. Values used twice are given once, and results used by a later line are intermediate
    steps. **Copy as LaTeX** puts it on the clipboard for amsmath, **Copy as Markdown** with
    simpler LaTeX (one `aligned` formula per block) for notes such as Obsidian or GitHub. The
    result is underlined twice in both.
  - **CALC** and **SOLVE** in the input's context menu and the Calculation menu.
  - **Sessions**: each saved as a file in the config directory after every change, with its
    variables, history and linear system; a session panel (Ctrl+O) to open, create, rename,
    duplicate and delete them, and the last one opened again on start. The history now keeps
    200 entries and can be cleared.

- `writeup` in the engine API: the history of a session as given, wanted and solution blocks.
  It recognises about twenty school laws by their units, rearranges them exactly, and splits
  sums and parts with a law of their own into steps of their own.
- `search_library` in the engine API: the constants and conversion units that fit a query, ranked.
  The conversion units now carry their names in English and German (`conversion_units` returns
  `{ latex, name }` per unit instead of the bare LaTeX).
- The setting **Angle unit** (`angleUnit`, degrees or radians) for the engine: with degrees,
  `\sin 30 = 0{,}5` and `\arctan 1 = 45`, and derivatives and `\arg` follow. `30^\circ` and
  `2\,\mathrm{rad}` keep their own unit in either setting. Radians stay the default.
- Angles in degrees, minutes and seconds: `2^\circ 20' 30''` or `2°20′30″`. A result that is an
  angle in degrees also comes in this form, in the new `forms` of the engine's response.
- The setting **Number format** (`numberFormat`) for the engine: Norm 1 and Norm 2 as on a Casio,
  Fix with 0 to 9 decimal places and Sci with 1 to 10 significant digits. The default writes
  numbers as before.
- The setting **Decimal prefixes** (`decimalPrefixes`) writes plain numbers with k, M, m, µ …:
  `4{,}7\,\mathrm{k}`.
- Results come in their other forms for a form switch: improper and mixed fraction, recurring
  decimal, decimal, exact root or π, and prime factors. The settings **Fractions**
  (`fractionForm`) and **Recurring decimals** (`recurring`) choose the order and whether the
  recurring decimal is offered. A recurring decimal can be typed as `0{,}1\overline{6}`.
- Complex numbers in polar form: the setting **Complex numbers** (`complexForm`) writes results as
  `r \angle \theta`, the other form is among the result's forms, and `2\angle 60` types one.
- A Basis-N mode in the engine (`baseN`): whole numbers in decimal, hexadecimal, binary or octal
  (setting `numberBase`), 32-bit two's complement, the prefixes d, h, b, o and the bitwise
  operations and, or, xor, xnor, Not and Neg. The result comes in all four bases.
- A matrix mode in the engine (`matrix`): matrices up to 10 × 10 stored under any name, with
  sums, products, powers, the transpose, the inverse, the determinant, the identity and the row
  echelon forms, exact and with the elimination as steps. A session lists its matrices in the
  order they were defined (`Session::matrix_definitions`), and `delete_variable` deletes a matrix
  as well.
- Statistics of a data list in the engine (`statistics`), with an optional frequency column: n,
  mean, sums, variances, standard deviations, minimum, quartiles, median and maximum, exact where
  the data are. With pairs x, y: their sums and means, the seven regressions of a Casio (linear,
  quadratic, logarithmic, two exponential, power, inverse) with r, and the estimates x̂ and ŷ.
  Two new modes, `statistics` and `distributions`, for the shells to come.
- The distributions of a Casio in the engine (`distribution`): normal density, cumulative and
  inverse, binomial and Poisson density and cumulative for a list of k, the discrete ones exact.
- The functions `\coth` and `\operatorname{arsinh}`, `arcosh`, `artanh` (also `\sinh^{-1}`), in
  every mode and in derivatives and the curve sketch.
- Products `\prod_{i=1}^{n}`, written like sums.
- The Casio's number functions: `nPr`, `\gcd` and `ggT`, `lcm` and `kgV`, `Int`, `Intg`, `Rnd`,
  `RndFix` and `divR` (division with remainder), each as `\operatorname{…}(a; b)`.
- All 47 physical constants of a Casio fx-991DE X in Physics mode (CODATA 2018), from the
  impedance of vacuum `Z_0` to the Rydberg constant `R_{inf}`, and a list of them by category
  for a picker (`constants`).
- Imperial and US units and the other conversions of a Casio: `ft`, `yd`, `mi`, `nmi`, `lb`,
  `oz`, `gal`, `psi`, `Btu`, `hp`, `°F`, `kn`, `lbf`, `mmHg` and more.
- Conversion to a unit with `▶` (`\blacktriangleright`): `5\,\mathrm{cm} ▶ \mathrm{in}`, in every
  mode, temperatures with their offset; and a list of the targets by the Casio's categories
  (`conversion_units`).
- Cubic and quartic equations are solved exactly (rational roots, Cardano, Ferrari), and
  polynomial equations of any degree up to 12 numerically on the whole real line instead of on
  [-100, 100]; the curve sketch finds polynomial zeros the same way.
- CALC and SOLVE in the engine: `free_variables` lists the letters of an input without a value,
  `calc` evaluates it with values given for them, and `solve_for` solves for a chosen letter
  (`F_1 = m_1 a_1` for `a_1`), from a start value with Newton's method where no exact way exists,
  ending with `L - R`.
- An equation without an unknown is checked: `2 + 3 = 5` gives *true* (*wahr*). Comparisons
  `<`, `>`, `\le`, `\ge`, `\neq` and chains such as `1 < 2 \le 2` do the same, and a `verify`
  mode (Prüfen) checks every input.
- Sessions in the engine: each keeps its variables and matrices, a history of up to 200
  calculations, what the modes hold, a name ("Sitzung 2026-11-01 14:30") and its times, and
  goes out and comes back as JSON with a format version (`export_session`, `import_session`);
  new, list, rename, duplicate and delete. The values are saved by their definitions and exact
  again after loading.
- `\mathrm{Ans}` and `\mathrm{PreAns}`: the last numeric result of a session and the one before,
  saved with the session.
- A request and response format for a remote engine (`api::dispatch`): a handheld can send any
  engine call as JSON, with its own settings and session, and gets the result and its updated
  session back.
- The setting **Digit grouping** (`digitGrouping`) writes long numbers as `12\,345{,}6`.
- Engineering notation for ENG and ←ENG in the engine (`engineering`): the exponent a multiple of
  three, moved by three per step. Responses carry the unit in LaTeX as `unitLatex`.
- A fifth mode, **Linear systems** (German **LGS**), for systems of linear equations, solved
  exactly with their Gauss steps. The **Equations** tab takes one equation per line (Enter
  solves, a second Enter or `\\` starts a new line). The **Matrix** tab takes `[A | b]` in a grid
  of up to 10 × 10 formula cells, empty cells being 0, or any size as JSON, pasted or loaded from a
  file. Switching tabs converts the system. The mode uses no session variables; the switch
  **Imaginary unit i** makes `i` the imaginary unit and allows complex coefficients. The setting
  **Unknown names** names the matrix columns `x_1, x_2, …` or `x, y, z, …`. The result and the steps are
  set in display style, so fractions stand at full size.
- The **Electronics** mode is back in the mode selector.
  - Its voltages and currents now sit beside their parts instead of piling up in the top left of
    the canvas. Ideal wires get no label; their values show in the inspector.
  - A changed precision or output setting shows in the circuit at once.
  - Voltages, currents, powers, resistances and conductances get a unit prefix: `4 mA` instead of
    `0,004 A`, `2 kW` instead of `2000 W`.
  - The buttons **U I P R** at the top right of the canvas choose which values the parts show
    (voltage and current to start with). Each part now also reports its resistance: the one given,
    or U/I for a power load.
  - The canvas moves (drag with the right or middle mouse button, or scroll) and zooms (Ctrl and
    scroll, or pinch); Home goes back.
  - The labels of parts and nodes no longer overlap each other, the parts or the wires. A label
    with no room beside its part moves to the next free spot, and a dashed line leads to it.
    While parts are dragged, the labels keep their spots as long as those stay free, and settle
    again when the parts are dropped.
  - A wire or part ending in the middle of an ideal wire is now connected to it (a T-junction).
    A new wire can also start there: a grid point on an ideal wire shows a handle under the
    pointer, and dragging from it draws the wire.
  - Wires run only across and down. A wire drawn on a slant becomes two at a right angle, turning
    the way the pointer first went; a dragged wire end stays in line with the other end.
  - The open dot at a terminal or wire end is gone once something is connected there; it shows
    again under the pointer, to drag from. This holds in the Kirchhoff mode too.
  - Resistors, wires and power loads count their voltage and current along the real current, so
    a part drawn the other way round no longer shows negative values.
- A seventh mode, **Kirchhoff**, for resistor networks worked out by the junction and loop rules.
  Draw voltage sources, current sources, resistors and ideal wires; the canvas shows the
  junctions, an arrow for each branch current, a voltage arrow beside each part while U is
  shown, and a clockwise arrow in each loop. The current arrows follow the real current, so the
  currents come out positive; clicking one reverses it and its current turns negative. The
  values of the parts follow the arrows: a resistor's power is what it takes, a source's what it
  delivers. Under the canvas, in a panel whose height can be dragged, are the
  junction and loop rules (with names, then with values), the matrix with its Gauss steps, the
  exact currents and the power balance; **Copy** gives the working as an `align*` block. Hovering
  a loop or a rule highlights its parts. The circuit is separate from the Electronics one but
  uses the same files, which now also keep the reversed current arrows.
- The Kirchhoff mode works out the **resistance between two points**: place the terminals A and
  B on a drawing of resistors and wires, and the working merges resistors in series and in
  parallel one step at a time (`R_{23} = R_2 ∥ R_3`, then `R_{123} = R_1 + R_{23}`) until
  `R_AB` is left, exact. While the pointer is on a step, the canvas shows the network after that
  merge, the merged resistor in place of the first one it stands for. Parts that carry no
  current (an open end, shorted, not connected to A and B) are left out with a note. A network
  that is not built from series and parallel, such as a bridge, is finished with the node rule
  and a test current of 1 A.
- The setting **Resistance between A and B** (`starDelta`) lets such a network be reduced by a
  **star–delta conversion** instead: *Delta → star* turns three resistors in a ring into a star
  around a new node (`R_{Y1} = R_1·R_2 / (R_1 + R_2 + R_3)`), *Star → delta* turns three that meet
  at a node into a ring, and the merging goes on until `R_AB` is left. Each conversion is one
  step of the working, and while the pointer is on it the canvas shows the new star or delta in
  place of the three resistors. *Node rule* (the default) keeps the test current, which also
  finishes a network that the conversions don't reduce.
- The circuit editor (Electronics and Kirchhoff) has **resistors at 45°**: <kbd>Shift</kbd>+<kbd>R</kbd>
  turns the selected resistors by 45° about their middle, and their terminals stay on the grid.
  The preview of a star–delta conversion draws the new resistors at 45° where that reaches their
  nodes, so a bridge's star or delta lies on its corners without extra wires.
- The Kirchhoff mode writes a fraction in its results with the decimal after it:
  `\frac{11}{5}\,Ω = 2.2\,Ω`, or `\frac{2420}{799}\,Ω ≈ 3.02879\,Ω` when the decimal does not end
  within the precision. This holds for the currents and for the resistance between A and B.
- The circuit editor (Electronics and Kirchhoff) draws a filled **junction dot** where three or
  more wires and terminals meet, as a schematic does.
- A new mode, **Geometry** (German **Geometrie**), for analytic geometry in space: points,
  vectors, lines, planes, spheres and circles, worked out exactly with their steps.
  - Objects are added with a form (a line by point and direction or by two points; a plane in
    parameter, normal or coordinate form or by three points; a sphere by centre and radius,
    centre and point or four points; a circle by centre, radius and normal or as a sphere cut by
    a plane), with formula cells for the coordinates, or typed with their name:
    `A(1|2|3)`, `g: X = A + \lambda \vec{u}`, `E: 2x + y - z = 4`, `K: (x-1)^2 + y^2 + z^2 = 9`.
    A typed name already in the scene replaces that object where it stands.
  - Objects have names of their own, apart from the variables. Points are written `(1|2|3)` or
    `(1;2;3)`, vectors as `pmatrix` columns or `\vec{a}`; `×` of two vectors is the cross
    product, `\cdot` the dot product, `|\vec{a}|` the length.
  - A 3D view draws the scene, each object in its own colour with its name; drag turns it, the
    wheel zooms, with *View from above* and *Reset*. The projection is computed in the engine, so
    the handheld can draw the same picture later.
  - Tasks: the relation of two objects (identical, parallel, intersecting, skew, contained,
    secant, tangent, intersection circle, Potenzebene, …), distance, angle (in the angle unit),
    foot of the perpendicular, reflection, midpoint, tangent and tangent plane, a plane in another
    form, length, surface, volume, circumference and area, and an expression with the scene's
    points and vectors. Answers are exact (`2\sqrt{2}`, `\arcsin(\frac{\sqrt{7}}{14}) ≈ 10.8934°`);
    a 2D scene gives 2D results.
  - The steps follow the step detail, in English and German, and **Copy** gives them as an
    `align*` block. The scene and the task are kept in the session.
- A setting **Systems of equations** chooses how the steps of a system are worked: **Gauss**
  (default), as taught at school, or **Gauss–Jordan**, as before.
- The handheld keeps **sessions**, as the desktop app does: MODE → Sitzungen creates, opens,
  renames, duplicates and deletes them, each a JSON file in `sessions/` on the SD card, written
  after every calculation. The session open last comes back with its history and variables after
  a restart, and the files are the desktop's, so a session can be copied between the two.
- The handheld keeps its **settings** across a restart, in flash (`settings.txt` in the
  simulator's SD folder).
- The handheld has a **clock**: SETUP → Verbindungen → Uhrzeit shows the date and the time, which
  can be typed by hand (TT.MM.JJJJ, HH:MM), and a time zone from a list (Berlin by default),
  each with its summer time, instead of a whole-hour offset.
- Once its clock is set, the handheld names a new session after the local time, as the desktop
  does ("Sitzung 2026-11-02 14:30"), and its sessions note when they were made and last used.
- SETUP → Verbindungen → WLAN on the handheld lists the **networks** in reach (signal, lock,
  saved), asks for a new one's password (typed on a USB keyboard or over serial, shown as stars
  until S⇔D), lists the **saved networks** to forget one, and shows the **status** (network,
  address, signal). The fan in the top bar is dim until connected. The simulator plays three
  made-up networks.
- The handheld's **WLAN** works on the dev kit, through its ESP32-C6: it joins the strongest saved
  network by itself, keeps up to 8 networks with their passwords, is off in the exam mode, and
  sets the **clock from the network** (NTP) while connected.
- The handheld **updates the firmware of its Wi-Fi chip** (the ESP32-C6) itself at boot, from an
  image in its flash (`firmware/c6`), when the chip runs another ESP-Hosted version.
- The handheld has **sheets** for the modes that need more than one input line, each cell typed
  with fractions and roots like the input, moved through with the arrows and EXE, which goes on
  to the next cell and works the sheet out from the last one:
  - **LGS** opens on the matrix `[A | b]` (3 × 3 to start; OPTN sets the rows and the columns up
    to 10, or switches back to typing equations). EXE gives the solution and the Gauss steps with
    their matrices, scrolled with ↑/↓ and ←/→.
  - **Wertetabelle** asks for f(x), g(x) (optional), from, to and step, and shows the table.
  - **Statistik** takes the data in columns x, y (for the seven regressions) and a frequency
    column; EXE gives n, the mean, the sums, the deviations, the quartiles and the regression.
  - **Verteilungen** shows the fields of the chosen distribution (OPTN → Typ); k takes one value
    or several separated by `;`, which gives a table.
  The session keeps what the sheets hold (Wertetabelle its f(x) and range). A result can show
  x̄ and ŷ. In Statistik, OPTN → Schätzwerte asks for an x and a y, and the regression's answer
  ends with ŷ and x̂ for them. F5 is OK in the simulator and on a USB keyboard: it edits a cell as it is.
- The handheld does **Kurvendiskussion**: EXE sketches the function typed, with the graph on the
  first page (grid, axes, the curve, dashed asymptotes, the zeros, extrema and inflection points
  named) and a page per section after it (Ableitungen, Definitionsbereich, Symmetrie, …),
  turned with ←/→. Each page opens on its **result** alone; ↓ shows the working, with its sign
  tables, and the result again at its end. OK on the graph starts **moving** it: the arrows
  pan, + and − zoom, 0 goes back; a finger drags it too. OPTN lists the sections
  and opens forms for the **tangent and normal** at x₀, the **area** between a and b (or the
  outer zeros) and a **table of values**; after a sketch of the same function the tangent,
  the normal or the hatched area is drawn on its graph. The session keeps the function. On the
  ESP32-P4 a sketch takes 0.3 to 2 s.
- The handheld's OPTN inserts more of the engine's functions: arsinh, arcosh, artanh and coth
  under Hyperbol. Fkt., the product ∏ under Analysis, nCr, nPr, ggT, kgV,
  Int, Intg, Rnd, RndFix and ÷R under Zahl, minutes and seconds of an angle under Winkel.
- The handheld has a **CONST** key (F6 in the simulator, Ctrl+U over serial) and OPTN →
  Konstanten in Physik: the 47 constants in their six categories, each with its symbol. A **CONV**
  key (F7, Ctrl+Y) and OPTN → Werkzeuge → In Einheit convert a value: the units by category,
  inserted after `▶`.
- The handheld's **Matrizen** mode works: typed inputs like `A^{-1}` or `\det A`, and OPTN →
  Definieren asks for a name and opens a grid of the size set by OPTN → Zeilen and Spalten (up
  to 10 × 10); EXE on its last cell defines the matrix. VARS lists the matrices by their size and
  opens one in the grid to change it. OPTN inserts the determinant, the transpose, the inverse,
  the identity and the row echelon forms.
- The handheld's **Basis-N** mode works: OPTN sets the base (DEC, HEX, BIN, OCT), kept with the
  settings and shown in the top bar, and inserts the prefixes d, h, b, o and the logic and, or,
  xor, xnor, Not, Neg.
- The handheld's **Prüfen** mode works: an (in)equality or a chain of them is *wahr* or *falsch*.
- **CALC** on the handheld (F8, Ctrl+B, OPTN → Werkzeuge): a form over the input asks for each
  letter that has no value, EXE works the input out with them; the values stay in the session.
  **SOLVE** (SHIFT+CALC) uses the same form and solves for the letter left empty, or, with all
  of them given, for the one at the cursor, starting from its value.
- **STO** on the handheld (SHIFT+VARS; F1 then F11 in the simulator): the shown result is kept
  in the session under a name typed, any letter with letters, digits and an index after `_`.
  **VARS** (F11, Ctrl+V) lists the session's variables with their values; one chosen is
  inserted, gets a new value in the form of CALC, or is deleted.
- The handheld's screen draws the signs its fonts lack as lines: ∈, ∉, ∪, ∖, →, ⇒, ⇔, ∨, ↗, ↘,
  ∅, ⌣, ⌢, ℝ, ℤ, ℏ and the conversion arrow ▶, so a limit shows its arrow; `\text` keeps its spaces. A product `\prod`
  shows as ∏, and nCr (`\binom`) as n over r in brackets, a template of its own in the editor. A matrix (`pmatrix`,
  `bmatrix`, `vmatrix`) shows as its cells in brackets. A degree
  written as a power (`30^\circ`, `^\circ\mathrm{C}`) is read as the sign °.
- The steps of Algebra and of the curve sketch's working are **German** when the language is
  German: every explanation of the algebra pipeline has its German text ("2 mit (x + 3)
  ausmultiplizieren", "Jeden Bruch auf den Hauptnenner bringen", "Erste binomische Formel").
  Before, only the steps of the LGS, the analysis and the electronics were.

### Changed

- The settings moved out of the side panel into their own window (Ctrl+.), grouped into General,
  Numbers, Algebra, Steps and Input. Settings for a single mode are always shown and say which
  mode reads them; the mode itself is switched in the top bar only, its description shows on
  hover there.
- The steps of a system follow the school Gauss method: triangle form without fractions
  (`2·II − 3·I`), common factors of a row divided out, then the unknowns found from the bottom
  row up. Each matrix shows the operations next to the rows they change, the pivot in colour,
  the rows just changed marked (`II′`) and coloured. Detailed works out each row entry by entry;
  Maximal explains swaps and checks the solution in every equation.
- With infinitely many solutions, the Gauss method sets the free unknowns to parameters
  `λ, μ, …`: `x = 1 - λ ∧ y = λ ∧ λ ∈ ℝ`.
- A system without a solution is no longer an error: its steps show the row `0 = c` that
  contradicts, and the result reads "no solution".
- Matrix entries with `π` or roots are written exactly in the steps, not as long decimals.

- Systems of equations accept subscripted unknowns such as `x_1, x_2` when they have no value;
  a subscripted variable with a value stays a coefficient. Single equations are unchanged.
- The unknowns of a system are ordered `x, y, z`, then `a … w`, with subscripts by number
  (`x_2` before `x_10`) and Greek letters last, instead of alphabetically.
- Complex solutions of a system with rational parts are written exactly (`\frac{1}{2} - 3i`), and a
  complex factor of a parameter is bracketed: `x = i - (1 + i) · λ`.

### Fixed

- In Imaginary mode `iy` is `i · y`; it was read as the variable `i` times `y`.
- A system whose coefficients mix roots and `π` (`\sqrt{2} y + z = -\pi`) was refused as not linear.
- In German, a power load too large for its circuit is reported as *keine Lösung: die Last ist zu
  groß* instead of in English.
- `\pi = 3` and `e = 2{,}72` are compared and come out *false*; the constant on the left was read
  as the name of a variable to define, which has no value. `e_1 = 5` still names a variable.

## [0.1.0] - 2026-10-29

The first release.

### Added

- Four modes: **Physics** (units and constants are known), **Algebra** (every letter is a
  variable), **Imaginary** (`i` is the imaginary unit) and **Analysis** (curve sketching).
- A MathLive formula editor and a LaTeX source view with a live preview. <kbd>\\</kbd> switches
  between them.
- Exact arithmetic with arbitrary precision. Units and dimensions are checked and converted, and
  results are named (`N`, `J`, …).
- Equations with one unknown, systems of linear equations with their Gauss steps, derivatives,
  integrals (also improper ones), limits, sums, binomials and determinants.
- Simplifying and factoring with the domain conditions that go with them.
- A curve sketch with an interactive graph, tangent and normal, area and value table.
- Five levels of step detail. A result can be copied as LaTeX, a whole calculation as an
  `align*` block.
- Session variables, history and settings, kept in the app's config directory.
- English and German interface.
- Packages for Linux (AppImage) and Windows (`.msi`).

[Unreleased]: https://github.com/iswearimmrtobi/quantus/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/iswearimmrtobi/quantus/releases/tag/v0.1.0
