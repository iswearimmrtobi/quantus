# Basis-N programmer view

## Context

`plans/feature-ideas.md` idea 13. Basis-N (`crates/quantus-core/src/basen.rs`) is the mode of
the Casio fx-991DE X: fixed at 32 bit, signed, and a result outside the word is an error.
Programmers and electronics students need other widths (8, 16, 32, 64), unsigned numbers, shifts
and rotations, and the picture of the bits. The IEEE-754 view, open in the idea, goes into
Basis-N as its own last phase and reuses the bit view.

Decisions (from the questions):

| Topic | Decision |
|---|---|
| Word size | Setting `wordSize`: 8, 16, 32, 64; default 32 (as now) |
| Signedness | Setting `signed`: true/false; default true (as now) |
| Overflow | Results **wrap** modulo 2ⁿ like a register; the response carries a warning "wrapped to n bits". A literal that does not fit the word stays an error |
| Shifts and rotations | Functions `shl`, `shr`, `sar`, `rol`, `ror` (`(x, n)`), plus operators `<<` and `>>` |
| `>>` | Arithmetic when signed, logical when unsigned (as C) |
| Bit view | Structured field in the response; the frontends draw it. Groups of four, bit indices |
| Interaction | Desktop: clicking a bit flips it and rewrites the input. Handheld: read-only |
| IEEE-754 | In Basis-N, own phase: `f32(…)`/`f64(…)` turn a decimal number into its pattern; at 32 and 64 bits the response breaks the pattern down into sign, exponent, mantissa and exact value |

Current state, for reference:
- Core: `basen.rs` (tokenizer, `Parser`, `fit()` = the 32-bit error, `literal()`, `write()`),
  `BaseNResult { value: i32, latex, forms }`. Dispatch in `eval/calculator.rs::evaluate_base_n`
  (L469). Settings `number_base` in `settings.rs` (L259). Forms `FormKind::{Dec,Hex,Bin,Oct}` in
  `forms.rs`. Response `api.rs::CalculationResponse`.
- Handheld: `firmware/ui/src/settings.rs` (`NumberBase` choice, persisted `number_base`),
  `tree.rs` (`OPTN_BASE_N`, `SOFT_BASE_N`), `soft.rs` (base keys), `render/bar.rs` (base in the
  bar), `firmware/engine/src/lib.rs` (settings mapping L507, `shown()` L382). The handheld shows
  no forms in Basis-N today.
- Desktop: `desktop/src/App.tsx` (`BASE_CHOICES`, `BASE_KEYS`, the DEC/HEX/BIN/OCT `Segmented`
  at L612, form chips), `desktop/src/api.ts` (`Settings`, `NumberBase`, `CalculationResponse`).
- Tests: `basen.rs` unit tests, `tests/basen.rs`, the generated set
  (`test/generator/gens/basen.py`, `test/cases/base-n/`, checker
  `examples/run_cases/basen.rs`), `firmware/ui/tests/{optn,soft_keys,screen}.rs`,
  `firmware/engine/tests/engine.rs`.
- Locales: `locales/en.json`, `locales/de.json` (`core.basen.*`, `app.mode.baseN*`,
  `hui.tree.base*`).

## Semantics

**Word.** A value is `bits: u64` masked to `width` bits. Signed reads it as two's complement,
unsigned as 0…2ⁿ−1. Every operation computes in `i128` from the read value, then wraps
(`mask`). Wrapping sets a flag; the response warns once.

| Width | Signed range | Unsigned range |
|---|---|---|
| 8 | −128…127 | 0…255 |
| 16 | −32768…32767 | 0…65535 |
| 32 | −2147483648…2147483647 | 0…4294967295 |
| 64 | −2⁶³…2⁶³−1 | 0…2⁶⁴−1 |

**Literals.** Decimal: up to the top of the range (signed: up to 2ⁿ⁻¹, so `-128` can be typed
in 8 bit); hex/bin/oct: any pattern of at most `width` bits (`FF` in 8 bit signed is −1).
Larger is an error naming the width: *"'1FF' does not fit 8 bits."*

**Division.** Truncates toward zero; signed or unsigned by the setting (`-1 ÷ 2` unsigned
8 bit is `255 ÷ 2 = 127`). `MIN ÷ −1` wraps to MIN with the warning.

**Shifts and rotations** (count `n` is read as a plain non-negative number; a negative count is
an error):

| Function | Meaning | n ≥ width |
|---|---|---|
| `shl(x, n)`, `x << n` | logical left | 0 |
| `shr(x, n)` | logical right (zeros in) | 0 |
| `sar(x, n)` | arithmetic right (sign in) | all sign bits |
| `x >> n` | `sar` if signed, `shr` if unsigned | as those |
| `rol(x, n)`, `ror(x, n)` | rotate within the word | `n mod width` |

Shifting bits out is not an overflow (no warning). Precedence of `<<`/`>>`: below `+ -`, above
`and` (as C). Written as `<<`, `>>`, `\ll`, `\gg` (MathLive types `\ll`/`\gg`). Arguments are
separated by `,` or `;`.

**Changing a setting** re-evaluates nothing by itself; the next result uses it. On the desktop
the shown result is re-evaluated as today when the base changes.

## Phase 1 — Core: word size, signedness, wrapping

- [ ] **1.1 Settings** (`crates/quantus-core/src/settings.rs`)
  - [ ] `enum WordSize { W8, W16, W32, W64 }` in `basen.rs`, serialised as the numbers
        `8`/`16`/`32`/`64` (`serde_repr` or a manual impl), default `W32`; `fn bits(self) -> u32`.
  - [ ] Fields `word_size: WordSize` and `signed: bool` (default `true`) next to `number_base`;
        re-export `WordSize`. JSON keys `wordSize`, `signed`.
  - [ ] Extend the serde test (L416/L433) and `tests/settings.rs` for the defaults and a
        round trip; an old settings file without the keys loads as 32/signed.
- [ ] **1.2 The `Word` type** (`basen.rs`)
  - [ ] `struct Spec { width: u32, signed: bool }` with `mask()`, `min()`, `max()` (as `i128`),
        `read(bits: u64) -> i128`, `wrap(v: i128) -> (u64, bool /*wrapped*/)`.
  - [ ] Parser carries `i128` values and `spec`; replace `fit()` by `self.wrap(v)`, which
        records `self.wrapped = true` when the value changed.
  - [ ] `literal(digits, base, spec)`: range check per the table above; error key
        `core.basen.literal_too_big` with the width.
  - [ ] Signed/unsigned division and `Not`/`Neg`/`xnor` via mask.
- [ ] **1.3 Result** (`basen.rs`)
  - [ ] `BaseNResult { bits: u64, spec, latex, forms, wrapped: bool }`; `fn value(&self) -> i128`.
  - [ ] `write(bits, spec, base)`: decimal signed or unsigned per spec; other bases the pattern
        without leading zeros (unchanged behaviour at 32/signed).
  - [ ] `pub fn evaluate(latex, base, spec)`.
- [ ] **1.4 Calculator** (`eval/calculator.rs::evaluate_base_n`)
  - [ ] Pass `Spec` from settings; `value` via `Decimal::parse(&value.to_string())` (u64 range
        does not fit `from_i64`); `warning: Some(msg!("core.basen.wrapped", width))` when wrapped.
- [ ] **1.5 Texts** (both `locales/*.json`)
  - [ ] `core.basen.literal_too_big`, `core.basen.wrapped`; remove `overflow_32_bits` once unused.
  - [ ] While here: `not_number` gets the base name from a locale key instead of the English
        word (German shows "hexadecimal" today).
- [ ] **1.6 Tests**
  - [ ] Unit tests: each width signed/unsigned, literal limits, wrap + warning flag, `-128` in 8 bit,
        unsigned division, `MIN ÷ −1`. Change the existing overflow test to the wrap behaviour.
  - [ ] `tests/basen.rs`: settings through the API, `warning` in the response.
  - [ ] Generated set: `gens/basen.py` gets `width`/`signed` per case, wrap instead of the
        overflow error, literal-too-big errors; the case format carries the two settings;
        `examples/run_cases/basen.rs` applies them. Regenerate `test/cases/base-n/`, run
        `run_cases`, update `test/results/summary.md`.

## Phase 2 — Core: shifts and rotations

- [ ] **2.1 Tokenizer**: `Token::Comma` (`,` `;`), `Token::ShiftLeft` (`<<`, `\ll`),
      `Token::ShiftRight` (`>>`, `\gg`); `<`/`>` alone stay an error. `\lt`/`\gt` pairs as typed by
      MathLive are covered.
- [ ] **2.2 Parser**
  - [ ] New level `shift()` between `and()` and `sum()`.
  - [ ] Functions in `unary()`/`atom()`: `shl shr sar rol ror` (case-insensitive, plain or
        `\operatorname{}`), each `( expr , expr )`; error keys for a missing argument/comma.
  - [ ] Shift count: negative is `core.basen.negative_shift`; counts ≥ width per the table.
- [ ] **2.3 Texts**: the error keys, `function.shl/shr/sar/rol/ror` names (en, de).
- [ ] **2.4 Tests**: unit tests per function × width × signedness, `>>` both ways, precedence
      (`1 + 1 << 2` = 8, `1 << 2 and 4` = 4), rotation by 0, by width, by width+1. Generator
      `shifts()` in `gens/basen.py` with a Python reference implementation.

## Phase 3 — Core: the bit view in the response

- [ ] **3.1 Type** (`api.rs` or `basen.rs`): `BitView { width: u32, signed: bool, bits: String }`,
      `bits` as `'0'/'1'` most significant first (the frontends group them). Optional
      `ieee: Option<IeeeView>` (filled in phase 6).
- [ ] **3.2 Plumbing**: `Calculation.bits: Option<BitView>`, set only by `evaluate_base_n`;
      `CalculationResponse.bits` with `skip_serializing_if = "Option::is_none"`; history entries
      keep it if they keep the response (check `session.rs`).
- [ ] **3.3 Form for copy**: a `FormKind::Bits` whose LaTeX is the grouped pattern
      (`\mathtt{0000\,1111}`) so the desktop chips and Ctrl+D reach it and it can be copied.
      Locale `app.form.bits`.
- [ ] **3.4 Tests**: `tests/basen.rs` checks `bits` for 8-bit −1, 64-bit 1, and absence in other
      modes; `tests/result_forms.rs` for `Bits`.

## Phase 4 — Desktop

- [ ] **4.1 API types** (`desktop/src/api.ts`): `WordSize = 8|16|32|64`, `Settings.wordSize`,
      `Settings.signed`, `BitView`, `CalculationResponse.bits`, `FormKind` `'bits'`; defaults in
      `SettingsDialog.tsx` (L133).
- [ ] **4.2 Mode bar** (`App.tsx` L612): next to DEC/HEX/BIN/OCT a `Segmented` 8/16/32/64 and a
      toggle Signed/Unsigned; both call `changeSettings` and re-evaluate the shown input as the base
      switch does.
- [ ] **4.3 Keys** (`BASE_KEYS`): `<<`, `>>`, `shl(`, `shr(`, `sar(`, `rol(`, `ror(` as
      `\operatorname{shl}\left(#?,\right)` etc.
- [ ] **4.4 Bit panel** (new `desktop/src/components/BitView.tsx`, styles in `styles.css`)
  - [ ] Under the result in Basis-N: rows of 32 bits (64 bit = two rows), groups of four, index
        under each group's low bit (as Windows calculator: 63 … 32 / 31 … 0), the sign bit marked
        when signed.
  - [ ] Click a bit: flip it (BigInt), write the new value in the set base (small helper
        `desktop/src/bits.ts`: pattern → hex/bin/oct, or signed/unsigned decimal), put it into
        the input, evaluate. Keyboard: focusable bits, Space flips.
  - [ ] Warning line for a wrapped result uses the existing `warning` display.
- [ ] **4.5 Texts**: labels (word size, signed, unsigned, bit, the tooltip "click to flip") in
      both locales.
- [ ] **4.6 Check**: `npm run typecheck`; run the app (`npm run dev`) and try every width, a
      flip at bit 63 in signed 64, and the form chip `Bits`.

## Phase 5 — Handheld

- [ ] **5.1 Settings** (`firmware/ui/src/settings.rs`): `choice!(WordSize { W8, W16, W32, W64 })`,
      `choice!(Signedness { Signed, Unsigned })`, fields, defaults, `Setting::WordSize`,
      `Setting::Signedness`, get/set, persisted keys `word_size`, `signed`.
- [ ] **5.2 Engine bridge** (`firmware/engine/src/lib.rs` near L507): map both into the core
      settings; pass `bits` and the forms through to the UI result.
- [ ] **5.3 Menus** (`tree.rs` `OPTN_BASE_N`): choices "Word size" (8/16/32/64) and
      "Signed/Unsigned"; group "Shift/Rotate" with `<<`, `>>`, `shl()`, `shr()`, `sar()`, `rol()`,
      `ror()`. `SOFT_BASE_N` stays as is (the six logic keys), unless a free slot appears.
- [ ] **5.4 Bar** (`render/bar.rs` L85): base plus width and sign, e.g. `Hex 16u` / `Dec 32s`.
- [ ] **5.5 Result display**: S⇔D in Basis-N (today only exact/decimal, `app.rs` L2748) cycles
      set base → other bases → bit view (→ IEEE in phase 6). The bit view is drawn in
      `render/` (new `render/bits.rs` or inside `screens.rs`): groups of four in the small font,
      16 or 32 bits per row depending on the screen width, with the high index at the row's left
      edge. Read-only.
- [ ] **5.6 Keys**: the comma for the two-argument functions is reachable in Basis-N (check
      `layer.rs`).
- [ ] **5.7 Texts**: `hui.tree.word_size`, `hui.tree.signed`, `hui.tree.unsigned`,
      `hui.tree.shift_rotate` in both locales.
- [ ] **5.8 Tests**: `firmware/ui/tests/optn.rs` (new entries), `screen.rs` (bar text, bit view
      screenshot), `engine/tests/engine.rs` (width + shift through the bridge). Check on the
      simulator, then on the board (see the P4 serial notes).

## Phase 6 — IEEE-754 view

- [ ] **6.1 Input** (`basen.rs`): functions `f32(x)` and `f64(x)` whose argument is read as a
      *decimal* number in any base setting: digits, `.` or `{,}`, optional sign and exponent
      (`\cdot 10^{n}` or `e`n). Rounded to nearest-even by Rust's `str::parse::<f32/f64>()`; result
      is the pattern. Needs width ≥ 32 resp. 64, else error `core.basen.float_needs_bits`.
- [ ] **6.2 Breakdown** (`basen.rs` → `IeeeView`), when the width is 32 or 64:
  - [ ] `sign`, `exponent_bits`, `exponent` (unbiased), `mantissa_bits`, `class`
        (`normal`/`subnormal`/`zero`/`infinity`/`nan`).
  - [ ] `exact`: the float's exact value as LaTeX, computed exactly (mantissa · 2^e as a `Value`
        fraction, written as a finite decimal; long ones cut to the precision setting with "…").
  - [ ] `typed` / `error`: when the result came straight from `f32(…)`, the typed number and the
        rounding error (exact − typed).
  - [ ] Filled into `BitView.ieee` for every 32/64-bit result (any integer pattern can be read as
        a float, e.g. `h3F800000` → 1.0).
- [ ] **6.3 Desktop**: a tab "Integer | IEEE 754" on the bit panel; the IEEE tab colours sign,
      exponent and mantissa, shows each as a number and the exact value + rounding error. Keys
      `f32(`, `f64(`.
- [ ] **6.4 Handheld**: OPTN entries `f32()`, `f64()`; the IEEE page as the last step of the S⇔D
      cycle at 32/64 bit (three coloured fields, then the exact value).
- [ ] **6.5 Texts**: sign, exponent, mantissa, the classes, exact value, rounding error, the error
      key (en, de).
- [ ] **6.6 Tests**: `f32(0.1)` = `3DCCCCCD` with exact 0.100000001490116119384765625; `f64(0.1)`;
      `-0`, `f32(1e39)` = infinity, smallest subnormal, NaN pattern `h7FC00000`; an 8-bit word
      with `f32` is the error. Generator `ieee()` in `gens/basen.py` using Python's `struct`.

## Phase 7 — Docs and wrap-up

- [ ] `docs/docs/guides/base-n.md`: word size, signedness, wrapping, shifts, bit view, IEEE section
      with examples; `docs/docs/reference/syntax.md` (Basis-N section), `reference/settings.md`
      (the two settings), `contributing/test-set.md` if the case format changed.
- [ ] `firmware/docs/features.md`, `design/handheld-menus.md` §3.5 (new OPTN entries, S⇔D cycle).
- [ ] `CHANGELOG.md` `## [Unreleased]`: one entry per phase as it lands (core, desktop, handheld,
      IEEE); note the change from overflow error to wrapping.
- [ ] `README.md` L69/L216 if they mention 32 bit.
- [ ] Remove idea 13 from `plans/feature-ideas.md` (and its references at L32/L324/L373), and
      this plan's pointer from `plans/plans.md` when all boxes are ticked.

## Verification

Per phase, before ticking it off:

```sh
cargo test --workspace
cargo clippy --workspace --all-targets
cargo test -p quantus-texts                     # every key in en and de
cargo run --release -p quantus-core --example run_cases   # Basis-N at 100 %
cd desktop && npm run typecheck && npm run dev  # phases 4 and 6: by hand
```

Handheld (phases 5 and 6): `firmware/ui` tests, the simulator (`firmware/sim`), then the board.

## Effort

S–M as in the idea: phases 1–3 about a day, desktop half a day, handheld a day, IEEE a day.
