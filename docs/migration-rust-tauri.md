# Quantus: full migration plan, Kotlin/Spring/Electron to Rust/Tauri

## Context

Quantus is a LaTeX-in/LaTeX-out physics calculator. Today: Kotlin backend (Spring Boot 4.1, Symja, Indriya, ~1,970 lines) behind a localhost REST API, an Electron + React UI, and a bundled jlink JRE with CDS archives and a warm-up thread to make startup tolerable. The goal is a Rust + Tauri app: fast startup, small bundle, no JVM, no local server, no second process.

### Decisions (user-approved unless marked)

| Topic | Decision |
|---|---|
| UI to engine | Tauri commands (`invoke`), no HTTP, port, CORS, health polling or watchdog |
| Migration style | Clean break: port module by module, no dual-engine running. Kotlin stays as read-only reference until cutover |
| Numeric core | **`dashu`** (exact `RBig`/`IBig`) + **`astro-float`** (π, e, non-whole roots) + in-house `Value`/`Decimal`. *Changed from Numerica*: Numerica is MIT but every backend is LGPL (GMP, malachite). Symbolica ruled out for now (paid licence, redistribution needs permission); revisit for F8 |
| Frontend | Keep React + MathLive + KaTeX unchanged except `api.ts` and the Electron bridge |
| Parity | Goldens extracted once from the Kotlin tests, plus a **one-shot oracle dump from Indriya** for the unit table (see M1) |

## Status

- Done: branch `rust-tauri`; Cargo workspace; `quantus-core::{error, num}` with 12 passing tests; golden JSON (`roundtrip`, `parse_errors`, `evaluate`, `sessions`, ~80 cases) in `crates/quantus-core/tests/golden/`.
- Done: M1 oracle extraction. `units_symbols.json` (715 known symbols, 187,802 unknown candidates), `units_format.json` (79 units), `quantity_cases.json` (81 operation cases). Kotlin suite green before and after; the temporary dumper was deleted.
- Done: M5 eval, session, api (`quantus-core::{eval,session,api}`, 21 unit tests plus `tests/golden.rs` over all of `evaluate.json` and `sessions.json`).
- Done: M4 LaTeX (`quantus-core::latex::{ast,lexer,parser,normalizer,numbers,serializer}`, 17 unit tests plus `tests/latex_golden.rs` over all 42 round trips, 2 labels and 9 parse errors).
- Done: M3 quantity (`quantus-core::quantity`, 12 unit tests plus `tests/quantity_golden.rs` over all 81 cases of `quantity_cases.json`).
- Done: M2 units (`quantus-core::units`, 29 unit tests plus `tests/units_oracle.rs`). The oracle test checks all 188,499 one to three character candidates against `units_symbols.json` (known or not, dimension string and vector, exact factor and offset, printed form) and all 79 units of `units_format.json` on every field.
- The user's uncommitted Kotlin/Electron startup work is untouched and becomes obsolete at cutover.
- Environment: Rust 1.94, Node 26, webkit2gtk-4.1 2.52, GTK3, librsvg, patchelf, rsvg-convert are present. No `cargo-tauri`; use `@tauri-apps/cli` v2 through npm. Tauri 2.12 needs rustc 1.90 (OK).

## Target layout

```
Cargo.toml                       workspace
crates/quantus-core/             engine library, no Tauri dependency
  src/{error,num,units,quantity,latex,eval,session,api}.rs|dirs
  tests/golden/*.json  tests/golden.rs  tests/units_oracle.rs
desktop/src-tauri/               Tauri 2 shell: commands, menu, tauri.conf.json, icons
desktop/src/                     React UI (kept; api.ts rewritten)
```

`quantus-core::api` holds the serde response structs (`CalculationResponse`, `VariableResponse`, `StepResponse`, `ApiError {error, position}`) so the Tauri crate stays a thin wrapper and the JSON shape is tested in core.

## Design of the parts still to port

### Units (replaces Indriya; the riskiest module)

Indriya behaviour the app relies on, and the Rust model:

- **Atom**: prefix + symbol (`km`, `mN`, `µm`, `°C`, `min`, `Pa`) with a dimension vector, an exact rational factor to the SI coherent unit (π powers for degree to radian), and an optional exact offset (°C: 5463/20).
- **Unit** = `Atom` or `Product(Vec<(Atom, power: i32, root: u32)>)`, mirroring Indriya's `ProductUnit` (same atom multiplied adds powers, `m/m` is `ONE`). The Atom/Product split matters: `simplificationTarget` only rewrites *products* (`kg·m/s²` to `N`, but `kJ`, `N`, `km/h` stay).
- **Dimension** = vector of 7 base exponents as small rationals (roots can make `m^(1/2)`); equality = same vector.
- **Lookup** (`UnitLookup`): plain symbols only (letters and `°`), longest/greedy prefix decomposition exactly as `SimpleUnitFormat` decides (`mN` millinewton, `min` minute, `Pa` pascal, `ms` millisecond; `e`, `i`, `m/s`, `m2` unknown).
- **Conversions**: exact `AffineConversion {factor: RBig, offset: RBig, pi_power}`; folded from the atom factors, no Indriya converter chain needed. Preserve: `km/h` to `m/s` is exactly 5/18, prefixes are powers of ten, °C to K has offset 5463/20, degree keeps π symbolic.
- **Registry**: ordered `PREFERRED_UNITS` list (base units, then N, Pa, J, W, C, V, F, Ω, S, Wb, T, H, lx, Hz, Gy, kat; first match per dimension wins), the F11 quantity-name table, `describe()`/`format()` for error messages.
- **Formatting**: Indriya `SimpleUnitFormat` output (`N`, `m/s`, `km/h`, `kg·m/s²`, `℃`, denominator grouping) is used in the `unit` JSON field and error messages (`m (length) and s (time) cannot be added.`), and `UnitLatex` renders products from structure (numerator and denominator lists, `^{p/r}` for roots, degree-sign handling, `\mu `/`\Omega ` escapes).
- **Dimension string**: the UI shows `dimension` (`App.tsx:240`, e.g. `[M]·[L]/[T]²`), so Indriya's `Dimension.toString()` format must be ported too.

### LaTeX (hand port, mechanical)

- Lexer: tokens NUMBER/IDENTIFIER/COMMAND/operators/brackets; decimal `9{,}81`, `9,81`, `9.81` all kept as the text `9.81`; `\,` `\;` `\quad` `\left` `\right` dropped; whitespace includes `~` and `&`; `°` starts an identifier; Greek commands are identifiers.
- Parser: the recursive-descent grammar and the identifier rule (variable with subscript, else longest unit run, else constant, else error), `unit_context` inside `\mathrm/\text/\si/\unit/\operatorname/\qty/\SI`, `\num`, brace-less arguments including `takeFirstDigit` (`\frac123`), `\sqrt[n]{}`, `\dfrac`, `\tfrac`, `\div`, `\ast`, Greek symbol mapping (`\mu`→µ, `\Omega`→Ω, `\omega`→ω).
- `FractionNormalizer`, `LatexNumbers` (`{,}`, scientific outside 1e-4..1e15, `\cdot 10^{n}`), `LatexSerializer` (precedence-derived brackets, `\,` before unit-only right operands, negation rules, power-base brackets for fractions/roots).
- **Error text**: `"<reason> (at position N+1: '<12-char excerpt>…')"`. The current `QuantusError::Parse` in `error.rs` lacks the excerpt; add `source` and fix `Display` when porting the parser.

### Evaluation and sessions

- `PhysicalQuantity { value: Value, unit: Unit }`: `+ - * /`, negate, integer and quantity powers (dimensionless exponent required; unit-bearing base needs a rational exponent), `root(n)`, `to(target)`, `simplify()`, offset-unit addition error, mismatch messages.
- `ExpressionEvaluator`, `Substitution` (decimal value + unit node, brackets via serializer), `LatexCalculator` (symbolic, substituted if variables used, result; single `result` step when the definition equals its own result; `alignBlock`).
- `Session` (ordered map, replace in place), `SessionStore` (default id `"default"`), `SessionCalculator.evaluate/define` (define rejects unlabeled or subscript-less labels).

### Tauri shell

- Commands: `evaluate(latex, session_id?, precision?)`, `list_variables(session_id?)`, `delete_variable(key, session_id?)`; state `Mutex<SessionStore>`; long computations (10,000 digits) run via `spawn_blocking`. `POST /variables`, `DELETE all` and `/health` are not exposed (the UI never calls them); `define` stays in core for tests.
- `api.ts`: replace `fetch` with `invoke`; keep `ApiError(message, position)` and the German "no connection" path removed. `App.tsx`: drop health polling and the `starting/offline` state (nothing to wait for), keep the shortcut handling.
- Native shell: read `electron/main.ts` for the exact 8 custom menu items and accelerators; port with `tauri::menu`, emit events to the webview; zoom/fullscreen/devtools items; `tauri-plugin-single-instance`, `-opener` (external links), `-clipboard-manager` or `navigator.clipboard`, `-dialog` for fatal errors; window size, min size, background colour in `tauri.conf.json`.
- CSP in `tauri.conf.json`: `default-src 'self'`; `connect-src ipc: http://ipc.localhost`; `style-src 'self' 'unsafe-inline'` (MathLive); `font-src 'self' data:`; `img-src 'self' data:`. Keep `scripts/copy-mathlive-fonts.mjs`.

## Work packages

Each package ends green (`cargo test`, clippy) before the next starts.

**M0 Foundations (done).** Branch, workspace, goldens, `num`.

**M1 Oracle extraction (Kotlin, one shot; done).** A temporary Kotlin test (deleted after) dumps JSON to `crates/quantus-core/tests/golden/`:
1. `units_symbols.json`: every 1 to 3 character candidate symbol (letters, µ, μ, Ω, Ω, °, ℃) resolved by `UnitLookup.find` to its structure (prefix, atom, dimension vector, exact factor and offset via `UnitConversions`).
2. `units_format.json`: `UnitRegistry.format`, `UnitLatex.format`, `Dimension.toString`, `quantityName` and `simplificationTarget` for a list of ~60 representative units and products.
3. `quantity_cases.json`: results of `PhysicalQuantityTest`/`PhysicsFormulasTest` plus edge cases, as structured operations (operands and units as factor lists), not LaTeX: M3 comes before the parser, and LaTeX-level cases already live in `evaluate.json`.

Findings that change M2/M3:
- The lookup knows no angle degree (`°` alone, `deg`), so no symbol carries a π factor; `piPower` is 0 everywhere. Drop the π-power from `AffineConversion` unless a unit needing it turns up.
- `a` is the year, `d` the day, `h` the hour; many 3-letter strings are prefix+atom (`acd`, `alm`).
- `km/h`, `m/s` etc. only exist as products; products of different prefixed atoms do not merge (`km·m`, `m·km`, `km/m`), but addition and conversion still work through the dimension. `km/m` is dimensionless without simplifying to `1`.
- `dimensionVector` is lossy for roots (`m^(1/2)` shows exponent 1); the `Dimension.toString()` string (`[L]^1:2`, unit format `m^1:2`) is the reference there.
- Some Kotlin error texts embed Symja's printout (`'Sqrt(2)'`); compare those by prefix.
- Division by zero yields a non-real value (`decimal: null`), no error at quantity level.
Run `./gradlew test` once to confirm the Kotlin suite is green before and after (baseline for the cutover).

**M1b Tauri de-risk spike (early, throwaway-able).** Scaffold `desktop/src-tauri` with a stub `evaluate` returning the milestone result; point it at the existing React UI. Verify in WebKitGTK: MathLive input and fonts, KaTeX rendering, CSP, shortcuts, Wayland/NVIDIA rendering (`WEBKIT_DISABLE_DMABUF_RENDERER` if needed), and that `tauri build` produces an AppImage on this machine (set `NO_STRIP=true` if linuxdeploy fails on this rolling distro). Record the size and cold-start numbers next to the Electron AppImage. Exit: go/no-go on WebKit rendering before the engine port consumes more time.

**M2 Units (done).** `dimension`, `atom`/prefix tables, `Unit` (Atom/Product), `lookup`, `conversions`, `registry`, `format`. Exit: an exhaustive test compares the Rust lookup with `units_symbols.json` (every candidate symbol) and the format outputs with `units_format.json`; ambiguity tests (`mN`, `min`, `Pa`, `ms`, `m2`, `m/s`) pass.

M2 findings that change M3/M4:
- Lookup rebuilds `SimpleUnitFormat`'s name tables in registration order (`units::lookup`), with Indriya's overrides, instead of splitting prefixes. Beyond three characters the table is extrapolated from the source, not the oracle (`mmol`, `kOhm`, `dacd`, `month`, `k°C`), and covered by unit tests only.
- A `Dimension` remembers its construction order (`N` prints `[L]·[M]/[T]²`, `kg·m/s²` prints `[M]·[L]/[T]²`); equality ignores it. `Unit::dimension()` folds the factors in order, as `ProductUnit.getDimension`.
- `Dimension::is_compatible` keeps Indriya's lossy fallback (`DimensionalModel.getFundamentalDimension` folds `(acc·base)^e` and drops roots), so `m^(1/2)` counts as compatible with `m`. M3 inherits that on purpose; drop it only together with the Kotlin behaviour.
- Products with an offset unit (`°C/s`) or a root (`m^(1/2)`) cannot be converted: `Unit::system_conversion` returns `UnsupportedConversion`. The message is not Indriya's text (that embeds the unit's `toString`); M3 decides whether any case needs the exact wording.
- Unit LaTeX (`units::latex`, including `symbol()` for the parser) moved from M4 into M2 because its oracle is in `units_format.json`.
- `Unit::to_string()` is the registry format (`1` for the dimensionless unit); `registry::{preferred_unit, simplification_target, quantity_name, describe}` are the Kotlin `UnitRegistry` API.

**M3 Quantity (done).** `PhysicalQuantity` on `Value` and `Unit`; exact conversion; error messages byte-identical to Kotlin. Exit: `quantity_cases.json` and the dimension/offset tests pass.

M3 findings that change M4/M5:
- Division by zero is refused with `Division by zero.` instead of Kotlin's non-real ComplexInfinity; the golden runner accepts that one deliberate deviation. M5 should check what `evaluate.json` expects for `1/0`.
- The oracle dump never executed the `simplify` operator (those cases record the operand unchanged), so the runner treats `op: simplify` as a no-op; real simplification is covered by `quantity.rs` unit tests (`W/A` to `V`, `V/A` to `Ω`, `kg·m²/s²` to `J`, `J/(N·m)` to `1`).
- `Value`'s `Display` prints `Sqrt(x)` for a power of ½, matching the Symja text embedded in the "rational power" error.
- `UnsupportedConversion` for products with offsets/roots keeps its own wording; no quantity case needed Indriya's.

**M4 LaTeX (done).** AST, lexer, parser, normalizer, numbers, serializer, parse-error excerpt (unit LaTeX is done, see M2). Exit: `roundtrip.json` and `parse_errors.json` pass (positions 0-based, message substrings match).

M4 findings that change M5:
- `QuantusError::Parse` now carries `source`; its message is `<reason> (at position N+1: '<up to 12 characters>…')`. Positions are character indices (Kotlin: UTF-16 units), identical for the BMP.
- `Node::Unit` holds the resolved `Unit` and the spelling that resolved it; `serializer::quantity(&Decimal, &Unit)` is the result renderer M5 needs. `Node::variable_key` and `Label::key` give the session key `name_index`.
- Kept from Kotlin on purpose: a parenthesised fraction as a power base (`(a/b)^2`) serialises without brackets, because the `Group` hides the fraction from the bracket check. Pinned in a serializer test; fix it only together with the Kotlin behaviour.
- The lexer accepts ASCII digits only (Kotlin `isDigit` also takes other Unicode digits); such input now fails as an unexpected character.

**M5 Eval, session, API structs (done).** Evaluator, substitution, calculator with steps and `alignBlock`, sessions, `api` module. Exit: `evaluate.json` and `sessions.json` pass through a `tests/golden.rs` runner; JSON field names/shape equal the Kotlin `CalculationResponse` (checked with a serde round-trip test against the old controller expectations).

M5 findings that change M6:
- `api::{evaluate, define_variable, list_variables, delete_variable}` take `&mut SessionStore` and return the serde structs or `ApiError {error, position}`; the Tauri commands only lock the `Mutex<SessionStore>` and call them. `ApiError` implements `Serialize`, so it can be the command's error type directly.
- `position` is 0-based and `null` for non-parse errors, as in Kotlin; `App.tsx:310` already adds 1, so nothing changes.
- `dimension` is the Indriya string of the *simplified* unit, so the milestone shows `[L]·[M]/[T]²` (N), not `[M]·[L]/[T]²`; check the UI does not rely on the second spelling.
- `evaluate.json` has no `1/0` case, so the deliberate `Division by zero.` deviation from M3 is only covered by the `num` and `quantity` unit tests.
- Precision arrives as `Option<i64>` so that `0` and negative values produce Kotlin's range message instead of a deserialisation error.
- The Rust engine has no thread-local state, so a command may evaluate on any thread; only the store needs the mutex.

**M6 Wire Tauri to core (done).** Real commands, state, error mapping to `{error, position}`; rewrite `api.ts`; remove health polling. Exit: milestone, variable define/reuse/delete, parse error position, dimension error text, all visible in the running app.

M6 findings that change M7:
- M1b was never run, so M6 also created `desktop/src-tauri` (Tauri 2.12, workspace member `quantus-desktop`, no bundling yet, `csp: null`). `npm run tauri dev` builds and starts the window on this machine; `@tauri-apps/api` and `@tauri-apps/cli` are npm dependencies.
- Commands: `engine_info`, `evaluate`, `define_variable`, `list_variables`, `delete_variable`; the `Engine` state is a `Mutex<SessionStore>`. `api.ts` maps the `{error, position}` rejection to `ApiError`; `Health`, `STARTUP_GRACE_MS` and the polling effect are gone (the status dot is a static "bereit").
- Verified through Tauri's mock-runtime IPC (`src-tauri/src/tests.rs`): milestone, define/reuse/delete, parse-error position, `1m + 1s` error. Not seen in the visible window: no screenshot tool worked here (grim/spectacle refused on this compositor), so the window rendering (MathLive, fonts, KaTeX, WebKitGTK) is still unchecked by eye; M1b's go/no-go belongs to the first manual look.
- Tauri 2 only allows commands from `tauri://localhost`; the IPC tests use that origin.
- A definition from literals has two steps (`symbolic`, `result`); only inputs that use variables get the `substituted` step.
- `window.quantus` (copy, shortcuts) is still the Electron bridge and is `undefined` under Tauri; copy falls back to `navigator.clipboard`, shortcuts do nothing until M7.

**M7 Native shell parity (done in code, manual checklist open).** Menu, accelerators, single instance, external links, clipboard, zoom/fullscreen/devtools, fatal-error dialog. Exit: manual checklist against the Electron app, item by item.

M7 findings that change M8:
- The shell lives in `src-tauri/src/shell.rs`; menu items forward a `quantus://shortcut` event (ids in `FORWARDED`, mirrored by `Shortcut` in `src/shell.ts`). `window.quantus` and `quantus.d.ts` are gone; `inShell` replaces the old bridge check.
- Tauri has no zoom/fullscreen/devtools roles, so those are custom items: zoom (Ctrl+0, Ctrl+Plus, Ctrl+-, 50-300 %) is tracked in a `Zoom` state because the webview has no getter, fullscreen is F11, devtools (Ctrl+Shift+I) only in debug builds. These accelerators are new or differ from Electron's defaults.
- Plugins: `clipboard-manager` (copy), `opener` (external links), `single-instance` (registered in `main()`, not in `app()`: it needs a valid D-Bus application id, which the mock runtime lacks). Capabilities: `clipboard-manager:allow-write-text`, `opener:default`.
- The fatal-error dialog uses `rfd` directly (gtk3) from `main()`: a failed `run` or any panic shows "Quantus startet nicht" and exits 1. There is no backend process to fail any more.
- Verified: menu contains every forwarded id (unit test), IPC tests, `cargo run` stays up. Not verified by eye: accelerators in the real window, second launch focusing the first, link opening, clipboard content. Run the checklist in the Verification section before calling M7 closed.

**M8 Packaging (done, manual checks open).** Icon pipeline (`tauri icon` or the existing `rsvg-convert` script), desktop entry (category Education, keywords, `StartupWMClass`), AppImage via the Tauri bundler; optionally `.deb`/PKGBUILD. Exit: runs from a clean directory; measured cold start and size compared with Electron+JRE.

M8 findings that change M9:
- `npm run bundle` (`tauri build`) produces `Quantus_0.1.0_amd64.AppImage` (102 MiB) and `.deb` (6.4 MiB, depends on the system WebKitGTK) under `target/release/bundle/`. The AppImage bundles WebKit libs, so it is about 46 % of the Electron+JRE AppImage (223 MiB); the bare binary is 17 MiB.
- Icons: `tauri icon build/icon.svg`, trimmed to the Linux set in `src-tauri/icons` (32, 64, 128, 128@2x, icon.png). `build/icon.svg` stays the source; the `icon` npm script (rsvg-convert) is only needed by electron-builder and goes at M9.
- The desktop entry comes from `src-tauri/packaging/quantus.desktop` (Handlebars; used for both AppImage and deb): Education, keywords, `StartupWMClass=dev.tobifrosch.quantus`. Whether that class matches the window on Wayland/X11 is unchecked; verify in a taskbar before relying on it.
- Cold start, AppImage run from a fresh temp directory under `GDK_BACKEND=x11`, until a window titled Quantus exists: 457-510 ms (3 runs, includes the FUSE mount). The old Electron AppImage in `desktop/release` opened no window here (stale Kotlin backend), so there is no like-for-like number; measure at M9 or drop the comparison.
- Not done: PKGBUILD/Flatpak (AppImage is fine), rendering by eye, signing/updates.


**M9 Cutover (done).** Run the Kotlin suite one last time; delete `src/`, Gradle files, `gradlew*`, `desktop/electron/`, `electron-builder.yml`, Electron/concurrently dependencies, jlink/CDS tasks, `desktop/hs_err_*.log`; update README (German), `.gitignore`, spec status; keep `spec-si-rechner-v2.md` and `docs/phase0-findings.md` (add a note that Symja/Indriya findings are historical and which Rust behaviour replaces them). Commit in reviewable steps (only when the user asks).

M9 findings:
- The Kotlin suite ran once more with `--rerun-tasks`: 118 tests, 0 failures, then `src/`, Gradle files, `desktop/electron/`, `electron-builder.yml`, `tsconfig.electron.json`, `build/icon.png`, the Electron/concurrently dependencies and the stray `hs_err` log were removed (Kotlin stays in git history). The obsolete startup work (CDS, CasWarmUp) went with them.
- npm scripts are now `dev` (`tauri dev`), `dev:renderer`/`build:renderer` (called by `tauri.conf.json`), `bundle`, `typecheck`. `cargo test --workspace` and `npm run typecheck`/`build:renderer` pass after the removal.
- README is rewritten for Rust/Tauri; the spec carries a banner and struck-through N1/N2a (N4 marked deferred); `docs/phase0-findings.md` is marked historical with the Rust replacement per finding.
- The cold-start comparison with Electron is dropped: the Electron build no longer exists, and the 457-510 ms AppImage figure from M8 stands alone.

**M10 Follow-ups (not part of the migration).** Optional thin `quantus-server` crate (axum) over `quantus-core` if self-hosting/Docker (N4) is still wanted; history and persistence (phase 6); F8 analysis (derive, integrate, solve, plot), where a CAS such as Symbolica gets re-evaluated for licence terms; physical constants (c, h, G).

## Risks and mitigations

| Risk | Mitigation |
|---|---|
| Unit symbol parity with Indriya (prefix/ambiguity edge cases) | M1 oracle dump plus exhaustive Rust comparison; do not hand-guess the table |
| `Value` is a small symbolic type, not a CAS (no like-term collection, `π·π` stays a product) | Fine for today's features, since results are only read as decimals; F8 will need a real CAS later |
| WebKitGTK rendering differences (MathLive, fonts, Wayland/NVIDIA) | M1b spike before the heavy port |
| Tauri AppImage on a rolling distro and size (bundles WebKit libs, so savings may be smaller than hoped) | Measure in M1b/M8; fall back to `.deb`/PKGBUILD/Flatpak if AppImage is poor |
| Rounding drift vs Kotlin (Apfloat truncation then HALF_UP) | Already mirrored in `Value::decimal`; goldens include `…778`, 50-digit 1/3, scientific notation |
| Complex numbers (`i`) only partly supported in Kotlin | Port current behaviour (`NotReal` error), no new feature |
| No side-by-side run in a clean break | Goldens + oracle are the parity guarantee; run the Kotlin suite once more at cutover |
| Obsolete uncommitted Kotlin/Electron startup work | Left untouched; dropped at M9, the user decides when |

## Verification

- `cargo test --workspace` (unit tests, goldens, oracle comparisons) and `cargo clippy --workspace --all-targets` clean at the end of every package.
- End to end in the running app: `F_1 = 2kg \cdot 10\frac{m}{s^2}` shows `F_1 = 20\,\mathrm{N}`, quantity name "force", the dimension string, three-step presentation with variables (`m_1 = 2kg`, `a_1 = 9{,}81\frac{m}{s^2}`), a parse error with a 1-based position, `1m + 1s` error text, `20°C + 5K` error, copy of the `align*` block, every menu shortcut, second instance focuses the first.
- Packaging: build the AppImage, run it from a fresh directory, compare cold start and size with the current Electron AppImage.
