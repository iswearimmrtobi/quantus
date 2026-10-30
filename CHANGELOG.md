# Changelog

All notable changes to Quantus. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

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
