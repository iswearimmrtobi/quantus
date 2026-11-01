# Changelog

All notable changes to Quantus. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

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
  echelon forms, exact and with the elimination as steps.
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
- A setting **Systems of equations** chooses how the steps of a system are worked: **Gauss**
  (default), as taught at school, or **Gauss–Jordan**, as before.

### Changed

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
