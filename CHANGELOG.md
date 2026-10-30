# Changelog

All notable changes to Quantus. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- A fifth mode, **Linear systems** (German **LGS**), for systems of linear equations, solved
  exactly with their Gauss steps. The **Equations** tab takes one equation per line (Enter
  solves, a second Enter or `\\` starts a new line). The **Matrix** tab takes `[A | b]` in a grid
  of up to 10 × 10 formula cells, empty cells being 0, or any size as JSON, pasted or loaded from a
  file. Switching tabs converts the system. The mode uses no session variables; the switch
  **Imaginary unit i** makes `i` the imaginary unit and allows complex coefficients. The setting
  **Unknown names** names the matrix columns `x_1, x_2, …` or `x, y, z, …`.
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
