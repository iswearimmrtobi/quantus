# Unit keys

State on 2026-11-04: proposed, not built. The keypad layout is still open.

## Context

Physik is the default mode and tile 1, but typing a unit goes through OPTN → Physik → Einheiten
→ quantity → unit ([`tree.rs`](../ui/src/tree.rs), `UNITS`): 4–5 presses, and more with a prefix.
The 53-key layout ([hardware.md](hardware.md) §4) is modeled on the fx-991DE X, which has no unit
input at all. [handheld-menus.md](../../design/handheld-menus.md) §9 still lists "which keys get their own key" as open.
The engine already reads units as plain letters in Physik (`kN`, `m/s^2`), so the problem is
the keypad and the UI. The parser is fine.

## Recommendation: a third legend layer, UNIT, plus a unit bar on the screen

### 1. A UNIT modifier key (the main fix)

Add a fourth legend next to SHIFT (yellow) and ALPHA (red). Call it UNIT and give it its own
colour, e.g. the lavender `#9186e8`. Then:

- **Function block (4×6 = 24 keys)**: print the 24 most-used units on it. A proposal, grouped so
  related units sit together:
  `m  s  g  A  K  mol` · `N  J  W  Pa  Hz  C` · `V  Ω  F  T  H  eV` ·
  `l  h  min  °C  bar  %`
  The exact choice should follow what students type most (Mechanik, E-Lehre, Thermo).
- **Digit block**: put the prefixes on the digits, in order:
  `7 8 9 → G M k`, `4 5 6 → m µ n`, `1 2 3 → p c d`, `0 → T`.
  `µ` gets a key of its own, which avoids the `u` = dalton clash.
- **Rule: a prefix keeps the layer on.** UNIT then a prefix stays in UNIT for the next key, so
  `kN` is UNIT 7 N, which is 3 presses. A plain unit like `N` is 2 presses.
- **Operators pass through.** ÷, ×, x² and ^ type as usual and keep the layer latched, so
  `m/s²` is UNIT m ÷ s x² (5 presses for a compound unit). Any digit-free, non-unit key or EXE
  ends the latch.
- **SHIFT+UNIT locks it** (like ALPHA-lock) for long unit strings.
- The editor shows the active layer in the top bar ("U" next to S/A), as it does for SHIFT/ALPHA.

This costs one physical key. The 4-key top row or the 4×6 block can give it up. OPTN entries
like CALC/SOLVE are candidates for moving to SHIFT.

Why not just use ALPHA letters? The case collisions (m/M, k/K, s/S, p/P, t/T) and the missing
symbols (Ω, µ, °C, eV) make plain letters ambiguous and slow. A unit layer prints the real
symbol on the key.

### 2. A unit bar on the screen (1 press for the common case)

A thin bar along the bottom of the calculation screen in Physik, with 4–6 units:

- **Context first**: after a number, show the units you used last in this session (MRU). The
  core's quantity names could suggest more later.
- **Tap it** on the touch panel, or press **F1–F4**: map the 4-key top row to the bar's slots,
  as on the Casio CG / HP Prime soft keys.
- In a long exercise you use the same 3–4 units over and over, so most units become one press.

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

1. **Spec**: write the layer, the legend table and the latch rules into `firmware/docs/hardware.md`
   §4 and `design/handheld-menus.md` §7.4, and close the §9 question.
2. **Keys** (`firmware/ui/src/key.rs`): add `Key::Unit` and `Key::Soft(u8)`. Give each new key a
   Ctrl code in `CONTROLS` (serial and sim) and a USB HID code in `hid.rs` (e.g. F1–F4 → Soft,
   F6 → Unit).
3. **Layer state** (`firmware/ui/src/app.rs`, next to `alpha` around line 1074): add a
   `unit: Layer { Off, Once, AfterPrefix, Locked }`. One table maps
   `(Key) → &'static str` LaTeX for units and prefixes, reusing the strings the `units!` /
   `PREFIXES` pages in `tree.rs` already insert. Move those strings into a shared constant so
   the two cannot drift.
4. **Unit bar**: keep an MRU list of units per session. Fill it when an inserted run of unit
   letters is committed by EXE; the editor tree in `editor.rs` already knows what was inserted
   from a unit layer or menu. Draw it in `render/` as a 480×24 band above the bottom edge, only
   in Physik. Its hit areas go through `render::target` so touch works (`touch.rs`).
5. **Top bar**: show "U" while the layer is active (`render/bar.rs`).
6. **Sim** (`firmware/sim`): map keys for UNIT and F1–F4, and draw the third legend in the key
   help if the sim shows one.
7. **Docs/CHANGELOG**: add an Unreleased entry.

## Verification

- Unit tests in `firmware/ui` for the layer state machine: `UNIT 7 N` gives `kN`, `UNIT m ÷ s x²`
  gives `m/s^2`, the latch ends on EXE or a digit, and SHIFT+UNIT locks.
- In `firmware/sim`, type `5 UNIT 7 N · 3 UNIT m` and check the result shows as energy in J.
  Then check the unit bar offers kN and m afterwards and a tap inserts them.
- `cargo test --workspace`, `cargo clippy --workspace --all-targets`.
