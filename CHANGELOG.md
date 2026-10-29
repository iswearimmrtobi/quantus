# Changelog

All notable changes to Quantus. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

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
