# Keypad PCB: two boards, one 8 × 8 matrix each

The small unit's keypad was one board, `firmware/2_8_keypad.epro2` (73 × 115 mm, two layers, 67
matrix keys and ON, two MCP23017, an 8-pin JST SH connector). At 115 mm it is past the 100 × 100 mm
of the cheap fab tier, so it is split into two boards, each with its own MCP23017 and its own
matrix. Both stay two-layer. The matrix itself is in [hardware.md](hardware.md) §3.

| Board | Keys | Size | Chip |
|---|---|---|---|
| Top | soft keys, control row with ON, arrows, function block: 47 matrix keys (big unit 55) and ON | 73 × 71 mm | U1, 0x20 (big unit 0x21) |
| Bottom | number block: 20 keys on both units | 73 × 44 mm | U1, 0x22 |

The cut runs between the last function row and the first number row, at y −2810 mil of the old
board (the rows' switch centres sit at −2618 and −3002), so no key is on both boards.

[`tools/keypad_pcb.py`](../tools/keypad_pcb.py) sets up the matrix of each board, writes the nets,
routes the board and checks it. The parts, the outline and the connectors are placed by hand in
EasyEDA (below); the script cannot add parts.

## Why one 8 × 8 matrix per board

- An MCP23017 has 16 I/O pins: 8 rows on GPA0–GPA7 (inputs with pull-ups) and 8 columns on
  GPB0–GPB7 (outputs) read 64 keys, every pin in use.
- Only the bus crosses the cut: 3V3, GND, SDA, SCL, KP_INT, KP_RESET. No row or column net leaves its
  board, so each board routes on its own, with its chip next to its keys.
- Still three chips in all: the top board's, the bottom board's, and chip 3 on the main board.

Two layers are enough: rows on the top layer, columns on the bottom, as the router prefers anyway.
A four-layer board would add a GND plane that I²C at 400 kHz does not need; a GND pour on both
layers does the same job here.

## The matrices

A row is a row of keys, a column the key's place in its row of its block, as before. The top board
has nine rows of keys for eight matrix rows, so the control row (five keys, ON is not in the matrix)
gets its own column, C7, and takes rows R0–R4; it lies directly below the two soft rows, so those row
nets only run one row further down.

Top board, small unit:

| | C0 | C1 | C2 | C3 | C4 | C5 | C6 | C7 |
|---|---|---|---|---|---|---|---|---|
| R0 | F1 | F2 | F3 | F4 | F5 | – | – | SHIFT |
| R1 | F6 | F7 | F8 | F9 | F10 | – | – | ALPHA |
| R2 | SYMB | a/b | √ | – | ▲ | – | – | UNIT |
| R3 | VARS | x² | xʸ | ◄ | OK | ► | – | OPTN |
| R4 | CALC | log | ln | – | ▼ | – | – | MODE |
| R5 | (−) | x⁻¹ | sin | cos | tan | π | – | – |
| R6 | ( | ) | = | ; | CONST | ENG | – | – |
| R7 | x | y | e | i | CONV | S⇔D | – | – |

Top board, big unit:

| | C0 | C1 | C2 | C3 | C4 | C5 | C6 | C7 |
|---|---|---|---|---|---|---|---|---|
| R0 | F1 | F2 | F3 | F4 | F5 | F6 | – | SHIFT |
| R1 | F7 | F8 | F9 | F10 | F11 | F12 | – | ALPHA |
| R2 | SYMB | a/b | √ | x² | – | ▲ | – | UNIT |
| R3 | VARS | xʸ | log | ln | ◄ | OK | ► | OPTN |
| R4 | CALC | UNDO | ▶unit | x³ | – | ▼ | – | MODE |
| R5 | (−) | x⁻¹ | sin | cos | tan | π | 10ˣ | – |
| R6 | ( | ) | = | ; | CONST | ENG | Σ | – |
| R7 | x | y | e | i | CONV | S⇔D | ∫ | – |

Bottom board, both units (GPA4–GPA7 and GPB5–GPB7 stay free):

| | C0 | C1 | C2 | C3 | C4 |
|---|---|---|---|---|---|
| R0 | 7 | 8 | 9 | DEL | AC |
| R1 | 4 | 5 | 6 | × | ÷ |
| R2 | 1 | 2 | 3 | + | − |
| R3 | 0 | , | ×10ˣ | Ans | EXE |

Chip pins, the same on both boards:

| Net | Chip pins |
|---|---|
| ROW0–ROW7 | GPA0–GPA7 (pins 21–28) |
| COL0–COL7 | GPB0–GPB7 (pins 1–8) |

The firmware addresses a key as (board, row, column). Which unit it is follows from the address of
the top board's chip (0x20 small, 0x21 big), so the GPB0 strap of the one-board design is gone.

## Connectors

- **J1 on the top board**, to the main board: as before, 8-pin JST SH: 1 3V3, 2 GND, 3 SDA, 4 SCL,
  5 GND, 6 KP_INT, 7 KP_RESET, 8 PWR_ON (the ON key).
- **J2 on the top board** and **J1 on the bottom board**, 6-pin JST SH (`SM06B-SRSS-TB`): 1 3V3,
  2 GND, 3 SDA, 4 SCL, 5 KP_INT, 6 KP_RESET. Check that the cable connects pin 1 to pin 1; ready-made
  SH cables come both ways.

KP_INT is shared: the firmware sets `IOCON.MIRROR` (INTA and INTB on both pins) and `IOCON.ODR`
(open drain) on both chips, and the one pull-up sits on the main board, as before. KP_RESET keeps its
pull-up R1 on the top board.

## Making the two projects in EasyEDA

Both start as copies of `2_8_keypad.epro2`; the original stays untouched.

`2_8_keypad_top.epro2`:

1. Delete SW49–SW68 and U2, C2 (schematic and PCB).
2. Board outline: y from 0 to −2810 mil.
3. U1 address: A0 (pin 15) to GND, A1 and A2 to GND, for 0x20. The big unit's board ties A0 to 3V3
   for 0x21.
4. U1 INTA (pin 20) and INTB (pin 19) both to KP_INT.
5. Add J2 (6-pin SH) at the bottom edge with the nets above.
6. On the `matrix_input` page, at least one ROW or COL net port must remain on U1; the script
   rebuilds them all from it.

`2_8_keypad_bottom.epro2`:

1. Delete SW1–SW48 (ON included), U1, C1, R1 and the 8-pin J1.
2. Board outline: y from −2810 to −4528 mil (or move everything up so it starts at 0).
3. Keep U2 and rename it U1. Its straps already give 0x22 (A1 to 3V3, A0 and A2 to GND).
4. U1 INTA and INTB both to KP_INT (on the old U2 they were not connected).
5. Add J1 (6-pin SH) at the top edge, and move C2 (renamed C1) next to U1.
6. One ROW or COL net port on U1 on the `matrix_input` page, as above.

On both boards: screw holes or bosses near the cut on each side, so the keys next to the cut do not
give.

## Routing

Done by the script's own router, not EasyEDA's: two layers on a 5 mil grid, eight directions, A*
from the routed part of a net to its next pad. If a net fails, it moves to the front and the whole
board is routed again.

- 10 mil tracks, vias 12 / 24 mil (the project's sizes), 10 mil clearance while routing (the rule
  asks 6 mil; the grid costs up to 3.5 mil), 20 mil from the board edge.
- The top layer prefers horizontal runs (the rows), the bottom layer vertical ones (the columns and
  the runs to the chip). Each fine-pitch pad of U1 and the connectors first gets a straight 70 mil
  escape stub, so later nets cannot box it in.
- Power (3V3, GND) is routed as 10 mil tracks like the rest. A GND pour on both layers is the one
  thing worth adding by hand.
- The checker works on the geometry, not the router's grid: every net is one connected piece, no
  copper of two nets closer than 6 mil, nothing near the board edge.

## Check it in EasyEDA

The script cannot open EasyEDA, so check its output by hand:

1. Open `2_8_keypad_<part>_routed.epro2`. If EasyEDA refuses it, say so; the source project is
   untouched.
2. PCB: *Design → DRC* must pass and the ratsnest must be empty. *Design → Update from schematic*
   must find nothing to change (schematic and PCB use the same nets).
3. Check the pin order of every connector against the cable and the main board.
4. Optional: a GND pour.

## Redoing it

```sh
python3 -I firmware/tools/keypad_pcb.py --part top --table               # only the matrix
python3 -I firmware/tools/keypad_pcb.py --part top --render top.png      # route, check, draw, write
python3 -I firmware/tools/keypad_pcb.py --part bottom --render bottom.png
```

Needs Python 3, numpy, and matplotlib for `--render`. The input is `2_8_keypad_<part>.epro2` and the
output `2_8_keypad_<part>_routed.epro2`. A change to the keypad layout goes into the layout tables at
the top of the script (and into `design/keypad-sizes.html`), and then the boards are made again.
`--table` also works without the projects; it then prints the matrix only.

## History

The first matrix was *logical*: R0 held all ten soft keys, R1 the control keys and the arrows, and so
on. Its rows and columns joined keys that sat far apart, so nothing could be routed. The second one
was the physical grid on the one board, 13 rows × 6 columns on two chips (rows on U1, columns on U2,
GPB0 of U1 the unit strap), routed by this script into `2_8_keypad_rerouted.epro2`. That board's
rows ran to one chip and its columns to the other, all over a board too long for the cheap tier,
which led to the split.
