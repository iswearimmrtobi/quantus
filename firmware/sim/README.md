# Quantus handheld simulator

The handheld's UI (`firmware/ui`) in a window on the PC, at twice the display's 480×320. It uses
the same band renderer as the board, so the window shows what the display will show. The real
engine (`quantus-core`, through `firmware/engine` as on the board) answers. With `--stub` the
stub answers instead: the input itself (with `≈` in front as a decimal), and an input with a `?` gives an error. In
Kurvendiskussion EXE sketches the function typed: the graph, then a page per section, ←/→
between them; OPTN has the sections and the forms for the tangent, the area and a table.

The screen works like a Casio: a top bar (mode, a spinner while busy, SHIFT/ALPHA, significant
digits, exact or decimal, angle unit, WLAN, Bluetooth, exam mode, battery with a bolt while
charging), the input from the top left and, after EXE, its result at the
bottom right. After a result, an operator goes on with it, ←/→/DEL edit the input again and any
other key starts a new one.

Needs SDL2 (`sdl2` on Arch, `libsdl2-dev` on Debian). Its own Cargo workspace, so
`cargo test --workspace` does not need SDL2.

```sh
cd firmware/sim
cargo run
cargo run -- --keys '3/4>+2^3\n' --png screen.png   # type keys, save the screen
cargo run -- --stub                                 # the stub instead of the engine
cargo run -- --mirror /dev/ttyUSB0                  # also show the picture on the board
cargo run -- --charging --busy                      # start charging, with the spinner turning
cargo run -- --sd ~/quantus-sd                      # another folder as the SD card
cargo run -- --tour                                 # a walk through the UI with the wallpaper
```

The folder `sd` (or the one after `--sd`) stands for the SD card; its `wallpapers` folder holds
the pictures for SETUP → Gerät → Hintergrundbild and its `sessions` folder the sessions, as on
the board ([sd-card.md](../docs/sd-card.md#what-goes-on-the-card)). With `--png` the sessions are
read but not written. `sd/` is in `.gitignore`. The settings are kept in its `settings.txt`, the
saved WLAN networks in `wlan.txt`.

WLAN is played: SETUP → Verbindungen → WLAN finds three made-up networks after 1.5 s
(*Quantus-Labor*, *FRITZ!Box 7590*, the open *Café Gast*), takes any password and connects
after a second. The clock is the PC's; setting it by hand on the Uhrzeit page moves only the
simulator's.

`--tour` plays a walk through the UI with a person's pauses between the keys (about two
minutes): it picks the first wallpaper and shows it everywhere, opens each SETUP page, goes over
both pages of mode tiles, solves `2x+3=11` and `x²−5x+6=0`, adds `3/4+1/6` and switches it to a
decimal, steps through the history, opens OPTN and calculates in Physik. The terminal names each
part as it starts. With `--mirror` the board shows it too. `firmware/ui/tests/tour.rs` plays the
same keys without the pauses.

The PC has no battery or remote engine, so F9 and F10 switch the charging bolt and the busy
spinner by hand.

The keys are printed when the window opens. The mouse is the finger: a click taps, a drag
scrolls. Every menu works by touch: a tap opens a line or picks a tile, ‹ at the left of the top
bar goes back, a list follows the finger up and down and turns its page when let go, the mode
tiles do the same sideways, and a number is set with − and + and OK. SETUP → Gerät → Touch
kalibrieren shows the board's calibration screen; in the window the mouse needs none.

LaTeX typed into the terminal (then Enter) becomes the input and is evaluated, for what the keys
cannot type yet. The editor reads it like a result, and the terminal shows the LaTeX that went to
the engine, so anything the editor does not know shows up there. With `--png`, `--latex <text>`
does the same after the keys.

## On the board (testing only)

With `--mirror <port>` the board is the simulator's screen: the UI and the engine run on the PC
and the display shows what the window shows. The board needs its `mirror` binary first; see
[mirror.md](../docs/mirror.md).

In `--keys`, `\n` is EXE and `>`/`<` move the cursor.

## Keys

The window shows the handheld's keypad (the big unit's layout of `design/keypad-sizes.html`) right
of the screen: click a key to type it, arrows and DEL repeat while held, and a key typed on the PC
keyboard lights its key. SHIFT, ALPHA and UNIT relabel the keys with the layer table of
`design/handheld-menus.md` §7.4.1: yellow for SHIFT, red for ALPHA (A–F, M on the digits), lavender for
UNIT (prefixes on the digits, units on the function block). One press serves the next key; SHIFT
then ALPHA or UNIT locks the layer (shown as `UNIT=` in the top bar, thick outline on the key), and
SHIFT is one-shot on top of a lock. In UNIT prefixes, units and the operators ÷ × x² ^ keep the layer.
A legend the UI does not carry out yet (CATALOG, UNDO, INS, ≈, …) is dimmed and does nothing, as are
SYMB, the soft keys and the keys without code. `--keypad=small` draws the small unit, `--no-keypad` leaves the pad out,
`--scale <n>` sets the window's scale (2 by default, `--scale 1` for a small monitor).
With `--png` the pad is only in the picture with `--keypad`.

The characters are the same over the board's serial console (`firmware/handheld`), and a USB
keyboard on the board types the same keys (`firmware/ui/src/hid.rs`).

| Key | PC | Calculator |
|---|---|---|
| `0`–`9`, letters | as typed | digits, variables |
| `,` `.` | either | decimal comma |
| `;` | Shift+, | between equations (`x+y=3;x-y=1` in Linear) and arguments |
| `+` `-` `*` `:` `=` `(` `)` | as typed | operators, `×`, `÷` |
| `/` | | fraction; a number before it becomes the numerator |
| `^` | | exponent |
| `_` | | index (`r_1`) |
| Enter | | EXE |
| Backspace | | DEL |
| Esc, Delete, Ctrl+X | | AC |
| Tab | | MODE: the twelve mode tiles on two pages (then 1–9, or the arrows and Enter) |
| F1 then Tab | | SHIFT+MODE: setup. A digit or ↑/↓ and Enter open an entry; each entry opens a submenu: a list of values or a number typed with the digits. ← or Backspace go back, Esc closes |
| arrows | | cursor; ↑/↓ outside a fraction step through earlier calculations |
| F1, Ctrl+F | | SHIFT (SHIFT, then √ is the n-th root) |
| F2, Ctrl+K | | ALPHA |
| F9, Ctrl+Z | | UNIT |
| F3, Ctrl+D | | S⇔D: the result exact or as a decimal |
| F4, Ctrl+N | | OPTN: the mode's options (units, prefixes, functions, …), menus as in the setup |
| F5 | | OK: on a sheet, edits the cell as it is |
| F6, Ctrl+U | | CONST: the physical constants by category (Physik) |
| F7, Ctrl+Y | | CONV: `▶` and a unit to convert the value to, by category |
| F8, Ctrl+B | | CALC: a form asks for the letters of the input, EXE works it out; after F1, SOLVE: solved for the letter left empty |
| F11, Ctrl+V | | after F1, STO: the result kept under a name typed |
| Ctrl+W | | √ |
| Ctrl+Q | | x² |
| Ctrl+P | | π |
| Ctrl+E | | e |
| Ctrl+S, Ctrl+O, Ctrl+T | | sin, cos, tan |
| Ctrl+L, Ctrl+G | | ln, log |
| Ctrl+A | | Ans |
| F9 | | simulator only: charging on/off |
| F10 | | simulator only: busy spinner on/off |

SDL reports only the unshifted key, so the shifted characters in the window follow a German
layout (Shift+7 is `/`, Shift+8 `(`, Shift+0 `=`, Shift++ `*`). The number pad works on any
layout.
