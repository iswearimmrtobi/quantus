# Unit keys

State on 2026-11-09: the UNIT layer (§1), the soft keys with the unit bar (§2) and SYMB are built
in the hui and the simulator; the order of the bar comes from `quantus-core`
(`units::suggest`, `api::unit_suggestions`). Not built: the SETUP switch for the bar's order.
The keypad is sketched in [hardware.md](hardware.md) §4 (two sizes, soft keys, a UNIT key); the
legends are not fixed.

## Context

Physik is the default mode and tile 1, but typing a unit goes through OPTN → Physik → Einheiten
→ quantity → unit ([`tree.rs`](../ui/src/tree.rs), `UNITS`): 4–5 presses, and more with a prefix.
The Casio-style keypad ([hardware.md](hardware.md) §4) is modeled on the fx-991DE X, which has no
unit input at all. [handheld-menus.md](../../design/handheld-menus.md) §9 still lists "which keys
get their own key" as open. The engine already reads units as plain letters in Physik (`kN`,
`m/s^2`), so the problem is the keypad and the UI. The parser is fine.

Two things shape the answer: touch is never the way in (every function is reachable from the
keys), and units and symbols matter more than text. Physik and Algebra are equally important.

## Recommendation: a third legend layer (UNIT), soft keys, and a unit bar on them

### 1. A UNIT modifier key (the fast path once the positions are known)

A third modifier next to SHIFT (yellow) and ALPHA (red), in the round control row. Give it its
own colour, e.g. the lavender `#9186e8`. Then:

- **Function block (18 keys of the three lower rows)**: print the 18 most-used units on it.
  `x²`, `xʸ`, `(`, `)` and `=` stay out, because they pass through (below). A proposal, grouped
  so related units sit together:
  `m  s  g  A  K  mol` · `N  J  W  Pa  Hz  C` · `V  Ω  F  T  H  eV`
  `l`, `h`, `min`, `°C`, `bar` and `%` come from the soft keys. The exact choice should follow
  what students type most (Mechanik, E-Lehre, Thermo), and the assignment of units to positions
  is still to be set on the keypad sketch.
- **Digit block**: put the prefixes on the digits, in order:
  `7 8 9 → G M k`, `4 5 6 → m µ n`, `1 2 3 → p c d`, `0 → T`.
  `µ` gets a key of its own, which avoids the `u` = dalton clash.
- **Rule: a prefix keeps the layer on.** UNIT then a prefix stays in UNIT for the next key, so
  `kN` is UNIT 9 N, which is 3 presses. A plain unit like `N` is 2 presses.
- **Operators pass through.** ÷, ×, x² and ^ type as usual and keep the layer latched, so
  `m/s²` is UNIT m ÷ s x² (5 presses for a compound unit). Any digit-free, non-unit key or EXE
  ends the latch. After `xʸ` a digit is an exponent, not a prefix, and ends the latch after it.
- **SHIFT+UNIT locks it** (like ALPHA-lock) for long unit strings. SHIFT is one-shot on top of
  a lock and returns to it, so in a UNIT lock SHIFT x² types x³ and stays in UNIT.
- **Replace**: pressing another modifier replaces the pending one; the same one cancels it.
- The editor shows the active layer in the top bar ("U" next to S/A), as it does for SHIFT/ALPHA.
- **At most three legends per key.** UNIT legends go on the function block and the digits;
  ALPHA carries only the variables A–F and M, and sits on the digits (hardware.md §4), not on
  the function block, so the function block has base, SHIFT and UNIT. The UNIT legend can sit on
  the front edge of the cap. The HP 50g has four legends per key, so it is possible, but not at 77 mm pitch.

Why not just use ALPHA letters? The case collisions (m/M, k/K, s/S, p/P, t/T) and the missing
symbols (Ω, µ, °C, eV) make plain letters ambiguous and slow. A unit layer prints the real
symbol on the key.

### 2. Soft keys with a unit bar (1 press for the common case)

Two rows of plain keys directly below the screen (2 × 5 on the small unit, 2 × 6 on the big
one), labelled at the bottom of the screen ([hardware.md](hardware.md) §4). In Physik the bar
shows units. A mode with no use for the keys hides the strips, and the keys are then inert.

- **Entries are whole tokens with their prefix.** `kW` is one entry that types k and W, and the
  history remembers `kW`, so a user's usual prefix comes first next time.
- **Row 1 holds the best guesses, row 2 the alternatives** of the same dimension (other prefixes
  and units). Slot 1 is always the top guess. A page key reaches more.
- **Touch is an extra only.** A tap on a label does the same as the key.

#### Which units: the context

Rule-based, from the dimensions the core already knows; no learning. Signals, strongest first:

1. **After `+`, `−` or `=`**: only units of the same dimension as the left side. `3 kW +` offers
   W, kW, MW, mW, hp. This is always correct.
2. **After `×` or `÷`**: units that combine with the operand into a named quantity. After
   `3 kW ·` it offers h, s, min (→ energy); after `5 N ·` it offers m (→ J) and s (→ impulse).
   The table is the pairs that give a named derived quantity: N·m, V·A, A·s, kg·m/s², and so on.
   The core's CONV categories (19, `handheld-menus.md` §5.4) are the starting point.
3. **The variable left of `=`**: `P =` suggests W, `F =` suggests N, `U =` suggests V, `ρ =`
   suggests kg/m³. A small letter-to-quantity table; it ranks below the units already in the line,
   because `P` can also be a pressure.
4. **Tie-break**: the units used recently in this session, then a default per mode (Physik:
   Mechanik, E-Lehre, Thermo).

After a result, the same function offers the ▶unit targets (same dimension).

Rules that keep it usable by feel:

- The bar recomputes after an operator, `=`, `(` or the end of a unit, never after each letter,
  and it does not reshuffle while the same unit is being typed.
- It filters nothing. The UNIT layer and OPTN still type any unit; a wrong one gets the engine's
  dimension error.
- A SETUP switch turns the smart order off and falls back to most recent first.

#### Symbols (SYMB)

A SYMB key opens pages of symbols on the soft keys, each symbol SYMB plus one key (a page key
for more): Greek letters (α β γ δ ε θ λ µ ρ σ τ φ ω, Δ Ω Σ), then math symbols (∞ ± ≤ ≥ ≈ ° ′ ″
∠). The first page depends on the mode: Physik leads with Δ ω ρ µ ε Ω °, Algebra with π ∞ ± ≤ ≥.
Recently used symbols come first; after `sin(` the `°` comes first.

### 3. Keep OPTN → Einheiten for the rare ones

The OPTN tree stays for rare units (Torr, ly, AU, mph, Å, …). Add CONV (▶ unit) to it later
(§5.4).

### Press counts, before and after

| Input | Now | UNIT layer | Unit bar |
|---|---|---|---|
| N | 4–5 | 2 | 1 |
| kN | 7–9 | 3 | 1 (if used recently) |
| m/s² | ~12 | 5 | 1 (if used recently) |

## Implementation (once the layout is agreed)

1. **Spec**: the layer, the legend table and the latch rules go into `hardware.md` §4 and
   `design/handheld-menus.md` §7.4; close the §9 question.
2. **Keys** (`firmware/ui/src/key.rs`): add `Key::Unit`, `Key::Symb` and `Key::Soft(u8)` (0..12;
   the small unit uses 0..10). Give each new key a Ctrl code in `CONTROLS` (serial and sim) and a
   USB HID code in `hid.rs`.
3. **Layer state** (`firmware/ui/src/app.rs`, next to `alpha` around line 1074): add a
   `unit: Layer { Off, Once, AfterPrefix, Locked }`. One table maps
   `(Key) → &'static str` LaTeX for units and prefixes, reusing the strings the `units!` /
   `PREFIXES` pages in `tree.rs` already insert. Move those strings into a shared constant so
   the two cannot drift.
4. **Suggestions** (`quantus-core`, near `quantity`/`api`, so the desktop's insert bar can use
   them): a function that takes the input tree up to the cursor and returns the ordered entries.
   The input is unfinished (`5 N ·` does not parse), so it walks the editor tree backwards from
   the cursor to the last operator and the dimension of the operand on its left. The letter
   table and the "combines into" table are data next to it. Tests are integration tests in
   `tests/`.
5. **Soft-key bar**: keep an MRU list of tokens per session, filled when an inserted run of unit
   letters is committed by EXE. Draw the strips in `render/` (two bands, 2 × 5 or 2 × 6 labels,
   from the screen profile), hide them per mode, and send hit areas through `render::target` so
   touch works (`touch.rs`).
6. **Top bar**: show "U" while the layer is active (`render/bar.rs`).
7. **Sim** (`firmware/sim`): map keys for UNIT, SYMB and the soft keys, and draw the third
   legend in the key help if the sim shows one.
8. **Docs/CHANGELOG**: add an Unreleased entry when it ships.

## Verification

- Unit tests in `firmware/ui` for the layer state machine: `UNIT 7 N` gives `kN`, `UNIT m ÷ s x²`
  gives `m/s^2`, the latch ends on EXE or a digit, and SHIFT+UNIT locks.
- Tests for the suggestions as a table of `(input before the cursor → expected top three)`:
  `3 kW +` → W, kW, MW; `3 kW ·` → h, s, min; `P =` → W; `5 N ·` → m, s.
- In `firmware/sim`, type `5 UNIT 7 N · 3 UNIT m` and check the result shows as energy in J.
  Then check the bar offers kN and m afterwards.
- `cargo test --workspace`, `cargo clippy --workspace --all-targets`.
