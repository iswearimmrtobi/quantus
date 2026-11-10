# Keypad PCB: matrix and routing

The EasyEDA Pro project of the small unit's keypad is `firmware/2_8_keypad.epro2` (73 × 115 mm, two
layers, 67 matrix keys, ON, two MCP23017, an 8-pin JST SH connector). `2_8_keypad_rerouted.epro2` is
the same project with the matrix rebuilt and the board routed; the original is untouched. Both are
made and checked by [`tools/keypad_pcb.py`](../tools/keypad_pcb.py). The matrix itself is in
[hardware.md](hardware.md) §3.

## Why it was reworked

The first matrix was *logical*: R0 held all ten soft keys, R1 the control keys and the arrows, R2
SYMB … ln, and so on. Rows and columns therefore joined keys that sit far apart: ROW1 touched keys on
four different rows of the PCB, COL0 ran from the soft keys to the last number row. Nothing could be
routed without crossing everything, and the chips were not connected at all.

## The new matrix: the physical grid

- **Row** = a row of keys on the PCB, counted from the top: 2 soft rows (R0, R1), the control row
  (R2), the six rows of the function block with the arrow cross (R3–R8), the four number rows
  (R9–R12). 13 rows.
- **Column** = the key's place in its row of its block (C0–C5). The function block is 6 wide, the
  others 5. The big unit has the same 13 rows and a 7th column, and its own table.
- The cell of every key is computed from the keypad layout, so the table cannot drift from the
  sketch (`design/keypad-sizes.html` shows it in each key's tooltip).

Chip pins (every MCP23017 pin does the same, so the pins follow the rows):

| Net | Chip pins |
|---|---|
| ROW0–ROW7 | U1 GPA0–GPA7 (pins 21–28) |
| ROW8–ROW12 | U1 GPB1–GPB5 (pins 2–6); GPB0 stays the unit strap, GPB6–7 free |
| COL0–COL5 | U2 GPA0–GPA5 (pins 21–26); the rest free |

Gone: ROW7 as "big unit only", COL6–COL9. A scan is six steps instead of ten.

## Edits to the project

Schematic (pages `soft_btn`, `control_btn`, `arrow_btn`, `function_btn`, `number_btn`,
`matrix_input`):

- the two net ports of each of the 67 switches renamed to the new ROWn / COLm, and the labels
  ("F1  R0C0") to the new cell; the page notes point to this file;
- `matrix_input`: the net ports of U1 and U2 moved to the pins above, ROW8–ROW12 added, COL6–COL9
  removed.

PCB:

- pad nets of the switches, U1 and U2; the net list (ROW8–ROW12 added, COL6–COL9 removed); the
  silkscreen text "SWn RxCy" of each key;
- all old ROW/COL tracks (205) and vias (10) removed;
- new copper: 887 tracks (about 2.7 m), 185 vias.

Everything else is as it was: outline, parts and their places, other silkscreen, rules, 3D models.
J1, C1, C2, R1 and the chips were not moved.

### Old and new cell of every key

| Switch | Key | Old | New |
|---|---|---|---|
| SW1 | F1 | R0C0 | R0C0 |
| SW2 | F2 | R0C1 | R0C1 |
| SW3 | F3 | R0C2 | R0C2 |
| SW4 | F4 | R0C3 | R0C3 |
| SW5 | F5 | R0C4 | R0C4 |
| SW6 | F6 | R0C5 | R1C0 |
| SW7 | F7 | R0C6 | R1C1 |
| SW8 | F8 | R0C7 | R1C2 |
| SW9 | F9 | R0C8 | R1C3 |
| SW10 | F10 | R0C9 | R1C4 |
| SW11 | SHIFT | R1C0 | R2C0 |
| SW12 | ALPHA | R1C1 | R2C1 |
| SW13 | UNIT | R1C2 | R2C2 |
| SW14 | OPTN | R1C3 | R2C3 |
| SW15 | MODE | R1C4 | R2C4 |
| SW17 | ▲ | R1C5 | R3C4 |
| SW18 | ◄ | R1C6 | R4C3 |
| SW19 | OK | R1C7 | R4C4 |
| SW20 | ► | R1C8 | R4C5 |
| SW21 | ▼ | R1C9 | R5C4 |
| SW22 | SYMB | R2C0 | R3C0 |
| SW23 | a/b | R2C3 | R3C1 |
| SW24 | √ | R2C4 | R3C2 |
| SW25 | VARS | R2C1 | R4C0 |
| SW26 | x² | R2C5 | R4C1 |
| SW27 | xʸ | R2C6 | R4C2 |
| SW28 | CALC | R2C2 | R5C0 |
| SW29 | log | R2C7 | R5C1 |
| SW30 | ln | R2C8 | R5C2 |
| SW31 | (−) | R3C0 | R6C0 |
| SW32 | x⁻¹ | R3C1 | R6C1 |
| SW33 | sin | R3C2 | R6C2 |
| SW34 | cos | R3C3 | R6C3 |
| SW35 | tan | R3C4 | R6C4 |
| SW36 | π | R3C5 | R6C5 |
| SW37 | ( | R3C6 | R7C0 |
| SW38 | ) | R3C7 | R7C1 |
| SW39 | = | R3C8 | R7C2 |
| SW40 | ; | R3C9 | R7C3 |
| SW41 | CONST | R4C0 | R7C4 |
| SW42 | ENG | R4C1 | R7C5 |
| SW43 | x | R4C2 | R8C0 |
| SW44 | y | R4C3 | R8C1 |
| SW45 | e | R4C4 | R8C2 |
| SW46 | i | R4C5 | R8C3 |
| SW47 | CONV | R4C6 | R8C4 |
| SW48 | S⇔D | R4C7 | R8C5 |
| SW49 | 7 | R5C0 | R9C0 |
| SW50 | 8 | R5C1 | R9C1 |
| SW51 | 9 | R5C2 | R9C2 |
| SW52 | DEL | R5C3 | R9C3 |
| SW53 | AC | R5C4 | R9C4 |
| SW54 | 4 | R5C5 | R10C0 |
| SW55 | 5 | R5C6 | R10C1 |
| SW56 | 6 | R5C7 | R10C2 |
| SW57 | × | R5C8 | R10C3 |
| SW58 | ÷ | R5C9 | R10C4 |
| SW59 | 1 | R6C0 | R11C0 |
| SW60 | 2 | R6C1 | R11C1 |
| SW61 | 3 | R6C2 | R11C2 |
| SW62 | + | R6C3 | R11C3 |
| SW63 | − | R6C4 | R11C4 |
| SW64 | 0 | R6C5 | R12C0 |
| SW65 | , | R6C6 | R12C1 |
| SW66 | ×10ˣ | R6C7 | R12C2 |
| SW67 | Ans | R6C8 | R12C3 |
| SW68 | EXE | R6C9 | R12C4 |

## Routing

Done by the script's own router, not EasyEDA's: two layers on a 5 mil grid, eight directions, A*
from the routed part of a net to its next pad, a failed net is moved to the front and everything is
routed again.

- 10 mil tracks, vias 12 / 24 mil (the project's sizes), 10 mil clearance while routing (the rule
  asks 6 mil; the grid costs up to 3.5 mil), 20 mil from the board edge.
- Top layer prefers horizontal runs (the rows), the bottom layer vertical ones (the columns and the
  runs to the chips). Each fine-pitch pad of U1, U2 and J1 gets a straight 70 mil escape stub first,
  so later nets cannot box it in.
- Power (3V3, GND) is routed as 10 mil tracks like the rest. A GND pour on both layers is the one
  thing worth adding by hand.
- The checker works on the geometry, not the router's grid: every net is one connected piece, no
  copper of two nets closer than 6 mil (tightest found: 11 mil), nothing near the board edge.

The bottom-layer pads of J1 sit under the soft key SW3 on the top; the layers do not touch, but no
via can go there.

## Check it in EasyEDA

The script cannot open EasyEDA, so the file has not been loaded by it yet:

1. Open `2_8_keypad_rerouted.epro2`. If it refuses, say so; the original is untouched.
2. PCB: *Design → DRC*, and the ratsnest must be empty. *Design → Update from schematic* must find
   nothing to change (schematic and PCB use the same nets).
3. The bottom-layer pin order of J1 follows the footprint as drawn (pin 1 at the left, seen from the
   top); check it against the main board and the cable, as the schematic note says.
4. Optional: a GND pour; decoupling caps C1/C2 sit about 12 mm from their chips and could move
   closer.

## Redoing it

```sh
python3 -I firmware/tools/keypad_pcb.py --table                 # only the matrix
python3 -I firmware/tools/keypad_pcb.py --render keypad.png     # route, check, draw, write
```

Needs Python 3, numpy, and matplotlib for `--render`. About two minutes. It always starts from
`2_8_keypad.epro2`, so a later change to the keypad layout is made in the layout tables at the top
of the script (and in `design/keypad-sizes.html`), then the board is made again.
