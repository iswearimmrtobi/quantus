<div align="center">

<img src="desktop/build/icon.svg" alt="Quantus icon" width="128" height="128">

# Quantus

### A calculator for physical quantities, with LaTeX in and LaTeX out

*Type a formula, press enter, and get the working: symbolic, substituted, solved.*

![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/Rust-2021-orange?logo=rust&logoColor=white)
![Tauri](https://img.shields.io/badge/Tauri-2-24c8db?logo=tauri&logoColor=white)
![React](https://img.shields.io/badge/React-19-61dafb?logo=react&logoColor=black)
![Release](https://img.shields.io/github/v/release/iswearimmrtobi/quantus)

[Download](#-download) •
[Features](#-features) •
[Building](#-building) •
[Documentation](#-documentation)

</div>

---

Quantus is a desktop calculator for SI units and symbolic maths. You write in a
MathLive formula editor (or straight in LaTeX), evaluate with <kbd>⏎</kbd>, and
get the result together with its working. The engine is a Rust library that the
Tauri app calls directly: no server and no child process.

```
m_1 = 2kg                       →   m_1 = 2\,\mathrm{kg}
F = m_1 \cdot 9{,}81\frac{m}{s^2}
    SYMBOLIC     F = m_{1} \cdot 9{,}81\,\frac{\mathrm{m}}{\mathrm{s}^{2}}
    SUBSTITUTED  F = 2\,\mathrm{kg} \cdot 9{,}81\,\frac{\mathrm{m}}{\mathrm{s}^{2}}
    RESULT       F = 19{,}62\,\mathrm{N}
```

The interface is in English and German (Settings → Language).

## 📥 Download

Get the latest version from **[GitHub Releases](https://github.com/iswearimmrtobi/quantus/releases/latest)**:

| | |
|---|---|
| 🐧 **Linux** | `.AppImage`, runs on most distributions |
| 🪟 **Windows** | `.msi`; needs WebView2, which Windows 10 and 11 include |

What changed between versions is in [`CHANGELOG.md`](CHANGELOG.md).

## ✨ Features

### 🧭 Modes

The mode is chosen in the top bar or in the settings panel. Each one decides
how letters are read; an example under the empty input shows what fits.

| Mode | What it is for |
|---|---|
| **Physics** *(default)* | Units and physical constants are known; results are checked, converted and named (`N`, `J`, …) |
| **Algebra** | Every letter is a variable: simplify, factor and solve, no units or constants |
| **Imaginary** | As Algebra, but `i` is the imaginary unit and results may be complex, shown as `a + bi` |
| **Analysis** | Enter `f(x) = …` and get the whole curve sketch beside an interactive graph |
| **Kirchhoff** | Draw a resistor network and get the junction and loop rules, their Gauss steps, every current and a power balance |

### ✍️ Input

- **Formula editor** (MathLive): `/` makes a fraction, `*` a `\cdot`, `^` and `_`
  super- and subscripts, `2kg` a number with a unit
- **LaTeX source** for exact input, such as `\qty{12}{cm}` or `\sqrt[3]{27}`, with a
  live preview underneath that grows with the formula
- Switch between the two at any time with <kbd>\\</kbd>; the content carries over both ways
- **Session variables**: an indexed label such as `m_1 = 2kg` defines a variable for the session
- Parser errors report the **position** in the input

### 🧮 Calculation

- **Exact numbers** with arbitrary precision (`dashu`, `astro-float`)
- **Units and dimensions** are checked and converted, including common units outside the SI
- **Maths beyond arithmetic**: equations with one unknown, systems of linear equations, derivatives, integrals
  (also improper ones), limits (also one-sided and at infinity), sums, binomials and determinants
- **Algebra**: simplifying and factoring fractions, roots, powers, logarithms and the
  trigonometric identities, with the domain conditions that go with them
- **Exact roots and π**, a setting: `\sqrt{12}` is shown as `2\sqrt{3}` and `\pi/2`
  as `\frac{\pi}{2}` instead of a decimal (off by default)

### 📈 Analysis

Derivatives, domain, symmetry, zeros, extrema, inflection and saddle points,
monotonicity and curvature, behaviour at the edges and poles with asymptotes and
an antiderivative, each with its working. Tabs above the results add the tangent
and normal, the area under the curve and a value table.

### 📋 Working and output

- **Step detail**, a setting from *None* (only the result) through *Brief* and
  *Standard* (what you would write in your notebook) to *Detailed* and *Maximal*
  (every intermediate line and the bookkeeping)
- Cancelled factors are struck through or shown in colour, as you prefer
- Copy a result as LaTeX, or the whole calculation as an `align*` block
- A variables panel and a history panel keep track of the session

### ⚙️ Settings

Precision, step detail, how results are written (binomial direction, completing
the square, denominator form, pq or abc formula), how letters are read, the mode
and the language. Settings are kept in the app's config directory.

## 🔨 Building

**You need:**

| | |
|---|---|
| 🦀 **Rust** | stable, edition 2021 |
| 🟢 **Node** | 20 or newer |
| 🐧 **Linux** | WebKitGTK 4.1 and the usual GTK development packages, see the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) |
| 🪟 **Windows** | the MSVC build tools and WebView2 (built into Windows 10 and 11), same link |

```sh
cargo test --workspace   # engine tests, golden files and Tauri IPC tests (see docs/docs/contributing/testing.md)

cd desktop
npm install              # once; also copies the MathLive fonts to public/fonts
npm run dev              # Vite dev server + Tauri window with hot reload
npm run typecheck        # check the TypeScript
npm run bundle           # build the AppImage and .deb (on Windows: .exe and .msi)
npm run bundle:appimage  # only the AppImage (Linux)
npm run bundle:msi       # only the .msi (Windows)
```

> 💡 **Tip:** `npm install` has to run in `desktop/`, not in the repository root.

Besides the ~500 `cargo test` tests, a generated set of 103,000 cases in `test/`
checks the engine broadly; [`docs/docs/contributing/testing.md`](docs/docs/contributing/testing.md) explains both.

<details>
<summary><b>📦 Packaging</b></summary>

<br>

`npm run bundle` runs `tauri build` and puts the AppImage and the `.deb` in
`target/release/bundle/`. The AppImage (~100 MB) ships WebKitGTK; the `.deb`
(~6 MB) depends on the system's WebKitGTK. The desktop entry comes from
`desktop/src-tauri/packaging/quantus.desktop`, and the icons are generated from
`desktop/build/icon.svg` with `npx tauri icon`.

```sh
cd desktop && npm run bundle
../target/release/bundle/appimage/Quantus_0.1.0_amd64.AppImage
```

On Windows the same command builds an NSIS installer (`bundle/nsis/*.exe`) and an
`.msi` (`bundle/msi/`). If WebView2 is missing, the installer downloads it.

You don't need a Windows machine for this: the *Release build* workflow
(`.github/workflows/release.yml`) runs the desktop tests and builds the AppImage
on Linux and the `.msi` on Windows, on every `v*` tag or by hand from the Actions
tab. The packages are attached to the run as the `quantus-linux` and
`quantus-windows` artifacts; on a tag they also go into a draft GitHub release.

</details>

<details>
<summary><b>🧱 How the code is laid out</b></summary>

<br>

| Path | What it does |
|---|---|
| `crates/quantus-core/src/num.rs`, `quantity.rs` | Numbers and `PhysicalQuantity` |
| `crates/quantus-core/src/units/` | Unit table, dimensions, conversion, naming |
| `crates/quantus-core/src/latex/` | Lexer, parser, AST, serializer |
| `crates/quantus-core/src/eval/` | AST evaluation, substitution, steps |
| `crates/quantus-core/src/algebra/` | Simplifying, factoring and solving with working |
| `crates/quantus-core/src/analysis/` | The analysis mode: curve sketch, graph, tangent, area, table |
| `crates/quantus-core/src/electro/` | The electronics mode (not in the app yet): materials, wires, circuit solver |
| `crates/quantus-core/src/kirchhoff/` | The Kirchhoff mode: drawing graph, branches, junction and loop rules |
| `crates/quantus-core/src/settings.rs` | User settings and modes |
| `crates/quantus-core/src/session.rs`, `api.rs` | Session variables and the app's JSON types |
| `crates/quantus-core/tests/` | Golden tests, carried over from the Kotlin tests |
| `desktop/src-tauri/` | Tauri shell: commands, menu, single instance, bundling |
| `desktop/src/` | React renderer: formula editor, LaTeX source, steps, panels |

Numbers, units and LaTeX I/O depend on each other in one direction only. The
Tauri commands know nothing but `api` and `SessionStore`.

</details>

## 📚 Documentation

The documentation is at **[quantus.tobifrosch.dev/docs](https://quantus.tobifrosch.dev/docs/)**,
built from [`docs/`](docs/) (run it locally with `cd docs && npm install && npm start`):

| | |
|---|---|
| 🚀 [Getting started](docs/docs/getting-started/) | Install, your first calculation, the concepts behind modes and variables |
| 🧭 [Guides](docs/docs/guides/) | Entering formulas, units, algebra, complex numbers, calculus, curve sketching |
| 📖 [Reference](docs/docs/reference/) | Syntax, units and constants, settings, keyboard shortcuts, known limits, FAQ |
| 🛠️ [Contributing](docs/docs/contributing/) | Architecture, building, the engine API, testing, translations |

Not on the site:

| | |
|---|---|
| 🗺️ [`plans/plans.md`](plans/plans.md) | Open work and plans |
| 📐 [`design/`](design/) | The specifications: the original one (German), the analysis and electronics modes, the step detail levels |
| 🎨 [`docs/static/mockups/quantus-ui-v2.html`](docs/static/mockups/quantus-ui-v2.html) | The UI mockup the desktop styles are based on |

**Languages.** Code, comments and documentation are in English; the specification in `design/`
is in German. The interface is available in English (the default) and German, see
[Translations](docs/docs/contributing/translations.md).

## 📜 License

Quantus is released under the [MIT license](LICENSE). The licenses of the libraries it is built
on are collected in [`THIRD-PARTY-LICENSES.md`](THIRD-PARTY-LICENSES.md), which ships with the app.

<div align="center">

---

<sub>Made with 🧮 for people who like to see the working.</sub>

</div>
