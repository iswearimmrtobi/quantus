# Quantus handheld simulator

The handheld's UI (`firmware/ui`) in a window on the PC, at twice the display's 480×320. It uses
the same band renderer as the board, so the window shows what the display will show. The real
engine (`quantus-core`, through `firmware/engine` as on the board) answers. With `--stub` the
stub answers instead: the input itself (with `≈` in front as a decimal), and an input with a `?` gives an error. Analysis
and Linear only evaluate plain inputs for now; curve sketches and systems have no screen yet.

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
read but not written. `sd/` is in `.gitignore`.

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
kalibrieren shows the board's calibration screen; in the window the mouse needs none. For testing,
until the keypad exists, the calculation screen has MENU, SETUP and OPTN buttons along the bottom.

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
| F2, Ctrl+K | | ALPHA (only shown for now) |
| F3, Ctrl+D | | S⇔D: the result exact or as a decimal |
| F4, Ctrl+N | | OPTN: the mode's options (units, prefixes, functions, …), menus as in the setup |
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
