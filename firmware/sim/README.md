# Quantus handheld simulator

The handheld's UI (`firmware/ui`) in a window on the PC, at twice the display's 480×320. It uses
the same band renderer as the board, so the window shows what the display will show. The real
engine (`quantus-core`) answers. With `--stub` the stub answers instead, as on the board: the
input itself (with `≈` in front as a decimal), and an input with a `?` gives an error. Analysis
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

The folder `sd` (or the one after `--sd`) stands for the SD card. Its `wallpapers` folder holds
the pictures for SETUP → Gerät → Hintergrundbild: BMP files of exactly 480×320 (any BMP colour
depth), for example from GIMP or `magick in.png -resize 480x320^ -gravity center -extent 480x320 out.bmp`.
`sd/` is in `.gitignore`.

`--tour` plays a walk through the UI with a person's pauses between the keys (about two
minutes): it picks the first wallpaper and shows it everywhere, opens each SETUP page, goes over
both pages of mode tiles, solves `2x+3=11` and `x²−5x+6=0`, adds `3/4+1/6` and switches it to a
decimal, steps through the history, opens OPTN and calculates in Physik. The terminal names each
part as it starts. With `--mirror` the board shows it too. `firmware/ui/tests/tour.rs` plays the
same keys without the pauses.

The PC has no battery or remote engine, so F9 and F10 switch the charging bolt and the busy
spinner by hand.

The keys are printed when the window opens.

LaTeX typed into the terminal (then Enter) becomes the input and is evaluated, for what the keys
cannot type yet. The editor reads it like a result, and the terminal shows the LaTeX that went to
the engine, so anything the editor does not know shows up there. With `--png`, `--latex <text>`
does the same after the keys.

## On the board (testing only)

With `--mirror <port>` the board is the simulator's screen: the UI and the engine run on the PC,
you type into the window, and the part of each band that changed goes over serial to the board, so the
display shows exactly what the window shows. Flash the board's `mirror` binary first and close
the serial monitor, since the port takes only one user:

```sh
cd firmware/handheld && cargo run --release --bin mirror   # flash, then quit the monitor
cd firmware/sim && cargo run -- --mirror /dev/ttyUSB0
```

The link runs at 921600 baud, about 90 KB/s, unless the board's `mirror` was built with
`QUANTUS_MIRROR_BAUD=2000000` (then pass `--baud 2000000` here too). `QUANTUS_SPI_MHZ` sets the
board's display clock (16 MHz by default) when trying faster ones. A full screen of the plain UI is a few KB; with a
wallpaper it is about 300 KB (2 bytes a pixel), so about 3 s, while a caret blink or a key is a
small rectangle. The real board draws to the display directly and is not slowed by this. When the
board resets it says `READY` and gets the whole screen again. The protocol is in `firmware/mirror`.

In `--keys`, `\n` is EXE and `>`/`<` move the cursor.

## Keys

The characters are the same over the board's serial console (`firmware/handheld`).

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
