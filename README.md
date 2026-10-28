<div align="center">

<img src="desktop/build/icon.svg" alt="Quantus icon" width="128" height="128">

# Quantus

### A calculator for physical quantities, with LaTeX in and LaTeX out

*Type a formula, press enter, and get the working: symbolic, substituted, solved.*

![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)
![Rust](https://img.shields.io/badge/Rust-2021-orange?logo=rust&logoColor=white)
![Tauri](https://img.shields.io/badge/Tauri-2-24c8db?logo=tauri&logoColor=white)
![React](https://img.shields.io/badge/React-19-61dafb?logo=react&logoColor=black)
![Status](https://img.shields.io/badge/status-pre--release-lightgrey)

[Features](#-features) •
[Building](#-building) •
[Usage](#-usage) •
[Interface](#-interface) •
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

> ⚠️ **Early and unreleased.** The engine was ported from Kotlin/Spring to
> Rust and the app from Electron to Tauri. The interface is in English and
> German (Settings → Language).

## ✨ Features

### 🧭 Five modes

The mode is chosen in the top bar or in the settings panel. Each one decides
how letters are read; an example under the empty input shows what fits.

| Mode | What it is for |
|---|---|
| **Algebra** *(default)* | Every letter is a variable: simplify, factor and solve, no units or constants |
| **Physics** | Units and physical constants are known; results are checked, converted and named (`N`, `J`, …) |
| **Imaginary** | As Algebra, but `i` is the imaginary unit and results may be complex, shown as `a + bi` |
| **Analysis** | Enter `f(x) = …` and get the whole curve sketch beside an interactive graph |
| **Electronics** | A schematic editor for passive DC circuits with live results |

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
- **Maths beyond arithmetic**: equations with one unknown, derivatives, integrals
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

### ⚡ Electronics

Resistors, wires, voltage and current sources, potentiometers and constant-power
loads, drawn in a schematic editor. Wires take their resistance from material,
length and cross-section. The solver is exact, never guesses a missing value, and
can solve backwards for one unknown (*which cross-section keeps the drop under 3 %?*).
Circuits are saved and loaded as JSON files.

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
cargo test --workspace   # engine tests, golden files and Tauri IPC tests (see docs/testing.md)

cd desktop
npm install              # once; also copies the MathLive fonts to public/fonts
npm run dev              # Vite dev server + Tauri window with hot reload
npm run typecheck        # check the TypeScript
npm run bundle           # build the AppImage and .deb (on Windows: .exe and .msi)
```

> 💡 **Tip:** `npm install` has to run in `desktop/`, not in the repository root.

Besides the ~500 `cargo test` tests, a generated set of 103,000 cases in `test/`
checks the engine broadly; [`docs/testing.md`](docs/testing.md) explains both.

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

You don't need a Windows machine for this: the *Windows build* workflow
(`.github/workflows/windows.yml`) runs the desktop tests and builds both
installers on every `v*` tag, or by hand from the Actions tab, and attaches them
to the run as the `quantus-windows` artifact.

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
| `crates/quantus-core/src/electro/` | The electronics mode: materials, wires, circuit solver |
| `crates/quantus-core/src/settings.rs` | User settings and modes |
| `crates/quantus-core/src/session.rs`, `api.rs` | Session variables and the app's JSON types |
| `crates/quantus-core/tests/` | Golden tests, carried over from the Kotlin tests |
| `desktop/src-tauri/` | Tauri shell: commands, menu, single instance, bundling |
| `desktop/src/` | React renderer: formula editor, LaTeX source, steps, panels, circuit editor |

Numbers, units and LaTeX I/O depend on each other in one direction only. The
Tauri commands know nothing but `api` and `SessionStore`.

</details>

## 🎹 Usage

| Key | Action |
|---|---|
| <kbd>⏎</kbd> | evaluate |
| <kbd>Ctrl</kbd>+<kbd>N</kbd> | new calculation |
| <kbd>Ctrl</kbd>+<kbd>J</kbd> | show or hide the working |
| <kbd>Ctrl</kbd>+<kbd>,</kbd> / <kbd>Ctrl</kbd>+<kbd>H</kbd> | variables / history panel |
| <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>C</kbd> | copy the result as LaTeX |
| <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>C</kbd> | copy the whole calculation as an `align*` block |
| <kbd>\\</kbd> or <kbd>Ctrl</kbd>+<kbd>L</kbd> | switch between formula editor and LaTeX source |
| <kbd>Esc</kbd> | leave the source · close a panel |

**Two ways to type.** The formula editor is the quick one. Arrow <kbd>→</kbd>
leaves an index or any other bracket, so `m_1` `→` `= 2kg` defines the
variable. The LaTeX source is the exact one: a backslash switches to it while
you type, and from then on what you write is what you get.

**Variables** always carry an index (`r_1`, `v_1`). An input with an indexed
label is a definition (`m_1 = 2kg`) and stays valid for the session. A label
without an index (`F = …`) only names the line.

**Analysis mode.** Enter a function as `f(x) = x^3 - 3x`, `y = …` or just the
expression. Polynomials, rational functions, `e^x`, `a^x`, logarithms, roots,
the trig functions and their inverses and `|x|` work, also combined. Results are
exact where school methods reach (`H(e | 1/e)`, `N(±√3 | 0)`) and marked as
numeric where they do not. The graph zooms with the wheel and moves by dragging;
clicking a point selects it in the list below and the other way round. Whether
quadratics are solved with the pq formula or the abc formula is a setting
(automatic: pq in German).

**Electronics mode.** Add parts from the palette and draw wires between them;
every change recalculates the circuit. <kbd>Space</kbd> rotates a part, and a marquee selects
several. Values that cannot be determined yet are greyed out with the reason.

## 🔌 Interface

The renderer calls Tauri commands (`desktop/src/api.ts`), which call
`quantus_core::api`:

<details>
<summary><b>Commands</b></summary>

<br>

```
evaluate         { request: { latex, sessionId?, precision? } }
                 → { input, latex, result, steps[], alignBlock,
                     value, unit, quantityName, dimension, variable }
define_variable  { request: { latex: "r_1 = 0{,}5m", … } } → Variable
list_variables   { sessionId? }               → [ { key, symbol, latex, value, unit } ]
delete_variable  { key, sessionId? }          forget a variable

analyze          { request: { latex, variable?, precision? } }
                 → { function, variable, sections[], points[], asymptotes[], period, plot }
sample_function  { request: { latex, xMin, xMax, samples, yMin?, yMax? } } → { segments }
tangent          { request: { latex, x0 } }   → { steps[], point, tangent, normal }
area_under_curve { request: { latex, from?, to? } } → { steps[], area, signed, regions[] }
value_table      { request: { latex, from, to, step } } → { rows: [ { x, y } ] }

solve_circuit    { request: circuit }         → results per element and node
load_circuit     { path }                     → circuit
save_circuit     { path, circuit }

get_settings / set_settings                   read and change the user settings
engine_info                                   → { application, precision }
```

</details>

The analysis and circuit commands are stateless; `docs/analysis-mode.md` §13 and
`docs/electronics-module.md` have the full shapes.

Errors come back as `{ error, position }`. For parser errors, `position`
(0-based) points at the spot in the input.

## 📚 Documentation

| | |
|---|---|
| 🧪 [`docs/testing.md`](docs/testing.md) | Every test in the repository and how to run it |
| 🗺️ [`docs/plans.md`](docs/plans.md) | Open work and plans |
| 📐 [`docs/spec-si-rechner-v2.md`](docs/spec-si-rechner-v2.md) | The specification (German) |
| ⚡ [`docs/electronics-module.md`](docs/electronics-module.md) | The specification of the electronics mode |
| 📈 [`docs/analysis-mode.md`](docs/analysis-mode.md) | The specification of the analysis mode (curve sketching) |
| 🪜 [`docs/step-detail.md`](docs/step-detail.md) | The detail levels of the working, with an example for each step |
| 🚧 [`docs/unsupported-inputs.md`](docs/unsupported-inputs.md) | What the engine supports and its known limits |
| 🎨 [`docs/quantus-ui-v2.html`](docs/quantus-ui-v2.html) | The UI mockup the desktop styles are based on |

**Languages.** Code and comments are in English. The interface is available in
English (the default) and German; the texts live in `desktop/src/i18n/` and, for
the electronics and analysis modules, next to their code in
`crates/quantus-core/src/electro/` and `crates/quantus-core/src/analysis/`. The
engine's own parser and algebra messages are English only. The specification is
in German; the other documentation is in English.

## 📜 License

Quantus is released under the MIT license, as declared in `Cargo.toml`.

<div align="center">

---

<sub>Made with 🧮 for people who like to see the working.</sub>

</div>
