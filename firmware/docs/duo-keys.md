# Duo: the input screen's keys per mode

The Duo's input screen ([duo.md](duo.md) §3) is a 480×320 touch screen in place of the keypad.
Its first version copied the handheld's keypad: two rows of soft keys, a control row, 16
function keys in two pages and the number block. On the bench (2026-11-25) that showed:

- the digits and operators were 39 px wide, too small to hit; the function keys (about
  62 × 49 px, ~10 × 7.5 mm) are the least a key may be;
- a screen that only copies a keypad wastes the reason to have one: every key can be a soft
  key, and each mode can show the keys it needs;
- SHIFT, ALPHA and UNIT belong at the top, taller;
- there were two page keys: the soft bar's ▸1/2 and the function block's 1/2.

This document goes through every mode and screen and ranks the keys it needs. It is the
specification for the input screen's layouts; nothing in it is built yet.

The arrows, OK, EXE and MENU are hardware buttons on the Duo ([duo.md](duo.md) §1) and are not
drawn. On the bench, which has no buttons yet, they come from the USB keyboard or the serial
line.

## 1. The frame

One frame for every context, at about 6.5 px/mm (65 px ≈ 10 mm):

```
┌───────┬───────┬───────┬───────┬───────┬───────┐
│ SHIFT │ ALPHA │ UNIT  │ OPTN  │  DEL  │  AC   │   top row, 44 px
├───────┼───────┼───────┼───────┬───────┬───────┬───────┐
│       │       │       │   7   │   8   │   9   │   ÷   │
├───────┼───────┼───────┼───────┼───────┼───────┼───────┤
│ mode keys: 3 × 4      │   4   │   5   │   6   │   ×   │
├───────┼───────┼───────┼───────┼───────┼───────┼───────┤   number block 4 × 4,
│ ~72 × 69 px           │   1   │   2   │   3   │   −   │   66 × 69 px
├───────┼───────┼───────┼───────┼───────┼───────┼───────┤
│       │       │   ▸   │   0   │   ,   │  Ans  │   +   │
└───────┴───────┴───────┴───────┴───────┴───────┴───────┘
```

- **Top row**: SHIFT, ALPHA, UNIT, then OPTN, DEL and AC, which every context needs.
- **Number block**: 4 × 4, each key larger than today's function keys. A context may put other
  keys there (Base-N: 0–F) or none (a graph).
- **Mode keys**: 12 cells, the mode's keys in the order of their rank (§2). They take the place
  of both the soft keys and the function block: a soft key is a mode key like any other.
- **One page key**: when a context has more than 12 mode keys, the last cell turns the page
  (▸ 1/2). There is no other: the soft bar's own pages (`soft::paginate`) become the pages of
  the mode keys.
- **Layers**: SHIFT, ALPHA and UNIT show their legends on the keys, as now; a key with nothing
  on the layer is faint. A modifier that is pending or locked is outlined.

### Ranks

| Rank | Where |
|---|---|
| **A** | page 1, always |
| **B** | page 1 while there is room, else page 2 |
| **C** | a later page, or only through SHIFT or OPTN |

Within a rank the order of the tables is the order on the screen: row by row, left to right.

## 2. The calculating modes

All of them share the number block (digits, `,`, + − × ÷, Ans) and these keys, unless the
mode says otherwise:

| Key | Rank | Why |
|---|---|---|
| ( ) | A | in nearly every input |
| (−) | A | negative numbers |
| a/b | A | fractions are the exact form |
| xʸ, x², √ | A | the most used templates |
| sin, cos, tan, ln, log | B | functions; SHIFT gives the inverses, eˣ, 10ˣ |
| π, e | B | constants |
| x⁻¹, S⇔D, ENG | C | through SHIFT or later pages |

### Physics (default mode)

The soft bar today shows the units that fit the input (`app.units.best`), and after a result
the forms and ▶unit conversions. On the Duo those are the first mode keys.

| Key | Rank | Note |
|---|---|---|
| the best units for the input (up to 6) | A | `app.units.best`, changes with the input |
| ( ) (−) a/b xʸ x² √ | A | the shared core |
| CONST | B | Physics only |
| ×10ˣ | B | has no `Key` today (§4) |
| more units (`app.units.more`) | B | the next page |
| sin cos tan ln log π | C | |
| **after a result:** S⇔D, ENG, ←ENG, ▶unit conversions | A | today's result bar, in its place |

```
┌───────┬───────┬───────┐
│   m   │   s   │  kg   │   the units for the input
│   N   │   J   │   W   │
│ ( )   │ (−)   │  a/b  │   drawn with ( and ) sharing a cell,
│  xʸ   │  √    │ ▸ 1/3 │   x² on page 2 (see below)
└───────┴───────┴───────┘
```

Open: whether ( and ) get one cell each (then x² moves to page 2) or share one cell split in two
halves of 36 px (too narrow by the frame's rule). The tables count them as two keys.

### Algebra

| Key | Rank | Note |
|---|---|---|
| x, y | A | the variables |
| =, ; | A | equations and systems |
| ( ) a/b xʸ √ | A | |
| x², (−) | B | |
| sin cos tan ln log π e | B | |
| ∫ d/dx Σ Π lim | C | today's bar |
| nCr \|x\| x! ggT kgV mod | C | today's bar |
| **after a result:** S⇔D, ENG, ←ENG | A | |

### Komplex

| Key | Rank | Note |
|---|---|---|
| i, ∠ | A | ∠ is SHIFT (−) today; here a key of its own |
| ( ) (−) a/b xʸ √ | A | |
| arg, Re, Im, \|z\| | B | today's bar |
| Konj., ▶r∠θ, ▶a+bi | B | not built yet (faint, as on the bar) |
| sin cos tan ln log π e | C | |
| **after a result:** the forms, ▶r∠θ, ▶a+bi | A | |

### Analysis (Kurvendiskussion)

Typing f(x):

| Key | Rank | Note |
|---|---|---|
| x | A | |
| ( ) a/b xʸ √ (−) | A | |
| e, ln, sin, cos | B | the usual curves |
| Abschnitte, Tangente, Fläche, Tabelle | B | today's bar |
| d/dx, ∫, tan, log, π | C | |

The answer pages and a graph take the whole pad (no number block):

```
┌───────┬───────┬───────┬───────┬───────┬───────┐
│ SHIFT │ ALPHA │ UNIT  │ OPTN  │  DEL  │  AC   │
├───────┴───────┼───────┴───────┼───────┴───────┤
│  ◄ Seite      │   Zoom +      │   Seite ►     │
├───────────────┼───────────────┼───────────────┤
│  Verschieben  │   Zoom −      │   Mitte (0)   │
├───────────────┼───────────────┼───────────────┤
│  Tangente     │   Fläche      │   Punkte      │
├───────────────┴───────────────┴───────────────┤
│                  Schließen                    │
└───────────────────────────────────────────────┘
```

The keys send what `sketch_key` and `graph_key` take today (←/→ turn pages, +/− zoom, 0 home,
OK pans, AC closes); the task keys open the forms that OPTN opens.

### Linear (LGS), Table (Wertetabelle), Statistics, Distributions

These are sheets: the arrows move between cells, OK edits one, EXE goes on. They need numbers
first.

| Key | Rank | Note |
|---|---|---|
| (−), a/b, √ | A | numbers in cells |
| x | A | Table's f(x) and g(x) only |
| i | A | Linear with i switched on (OPTN) only |
| ; | A | Distributions' k with several values only |
| ( ) xʸ x² | B | |
| Zeile +, Zeile − | B | Statistics; today in OPTN |
| sin cos tan ln log π e | C | Table's f(x) |

Linear with typed equations (OPTN → Eingabe) is like Algebra: x, y, =, ; first.

### Matrix

| Key | Rank | Note |
|---|---|---|
| A, B, C, D, E, F, M | A | the names, keys of their own (ALPHA 7 8 9 4 5 6 1 today) |
| Definieren, Bearbeiten | A | today's bar |
| det, Aᵀ, A⁻¹ | A | |
| REF, RREF, Einheitsm. | B | |
| ( ) (−) a/b xʸ | B | |
| √ x² sin cos tan … | C | |

That is 12 A keys: page 1 holds exactly them, everything else is page 2.

### Base-N

The number block shows the hex digits; + − × ÷ move to the mode keys.

```
┌───────┬───────┬───────┬───────┬───────┬───────┬───────┐
│  DEC  │  HEX  │  BIN  │   C   │   D   │   E   │   F   │
│  OCT  │  and  │  or   │   8   │   9   │   A   │   B   │
│  xor  │ xnor  │  Not  │   4   │   5   │   6   │   7   │
│  Neg  │ ( )   │ ▸ 1/2 │   0   │   1   │   2   │   3   │
└───────┴───────┴───────┴───────┴───────┴───────┴───────┘
page 2: + − × ÷ ( ) Ans , and the shifts (OPTN today)
```

- The base set is lit (as `soft::is_set` lights it on the bar today).
- Digits that are not digits of the base set are faint and do nothing: BIN leaves 0 and 1, OCT
  0–7, DEC 0–9.
- The block is laid out as hex keypads are: 0 at the bottom left, F at the top right.

| Key | Rank |
|---|---|
| 0–9, A–F | A (in the block) |
| DEC HEX BIN OCT | A |
| and or xor xnor Not Neg | A |
| + − × ÷ | B |
| ( ) Ans | B |
| shifts and rotations | C (OPTN) |

Open: whether + − × ÷ belong on page 1 instead of xnor and Neg, which are rarer.

### Verify (Prüfen)

| Key | Rank |
|---|---|
| = ≠ < > ≤ ≥ | A (today's bar) |
| ( ) (−) a/b xʸ √ | A |
| x, y, sin cos tan ln log π e | B |

## 3. Screens that are not modes

| Screen | Input screen | Note |
|---|---|---|
| Mode menu | number block only (1–9, then 10–12 by touch on the main screen) | tiles are tapped on the main screen |
| Menu lists (OPTN, SETUP, sessions), catalogues, VARS | number block only | the numbered lines; picked mostly by touch |
| Choice, number entry | number block, (−) where allowed | |
| Text entry: names, Wi-Fi password, STO, catalogue search | a letter keyboard over the whole pad (below) | today only A–F and M can be typed without a USB keyboard |
| Messages, notes, info | AC only | any key returns today |
| Calibration | its crosses (already built) | |
| SYMB pages | the 12 mode keys are the symbols; page key turns the page | as today's SYMB soft pages |
| Running app | the app's soft keys first, then the number block | 2048 and Snake: 8/4/6/2 as arrows; Sudoku: 1–9 |

The letter keyboard:

```
┌────┬────┬────┬────┬────┬────┬────┬────┬────┬────┐
│ q  │ w  │ e  │ r  │ t  │ z  │ u  │ i  │ o  │ p  │   48 × 64 px: narrower than the
│ a  │ s  │ d  │ f  │ g  │ h  │ j  │ k  │ l  │ DEL│   frame's rule; a keyboard is read
│ ⇧  │ y  │ x  │ c  │ v  │ b  │ n  │ m  │ .  │ -  │   as a whole, and phones use
│ 123│ ä  │ ö  │ ü  │    Leertaste     │ AC │ OK │   smaller keys
└────┴────┴────┴────┴──────────────────┴────┴────┘
```

German layout (QWERTZ) when the language is German, QWERTY otherwise. 123 switches to digits
and signs. The keys send `Key::Char`, which text fields already take.

## 4. Open questions

- **Keys without a `Key`**: ×10ˣ (printed on both keypads, does nothing), ▶unit (big keypad) and
  ON. ×10ˣ needs one before it can rank B in Physics.
- **UNIT outside Physics**: `layer::resolve` types units in every mode. The input screen could
  leave UNIT out (or draw it faint) where a mode has no use for units: Base-N, Matrix, Verify,
  the sheets.
- **ALPHA**: with A–F and M as keys of their own in Matrix and Base-N, and the letter keyboard
  for text, ALPHA has little left to do: `%` and RanInt#. Its top-row cell could go to another
  key.
- **( and )**: one cell each, or one cell split; see Physics.
- **Base-N's page 1**: + − × ÷ against xnor and Neg.
- **Hardware buttons**: if EXE, the arrows and MENU end up not being buttons after all, the
  frame needs room for them.
