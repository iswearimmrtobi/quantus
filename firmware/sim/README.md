# Quantus handheld simulator

The handheld's UI (`firmware/ui`) in a window on the PC, at twice the display's 480×320. It uses
the same band renderer as the board, so the window shows what the display will show. There is no
engine yet: the stub answers with the input itself (with `≈` in front as a decimal), and an
input with a `?` gives an error.

The screen works like a Casio: a top bar in the mode's color (mode, SHIFT/ALPHA, significant
digits, exact or decimal, battery), the input from the top left and, after EXE, its result at the
bottom right. After a result, an operator goes on with it, ←/→/DEL edit the input again and any
other key starts a new one.

Needs SDL2 (`sdl2` on Arch, `libsdl2-dev` on Debian). Its own Cargo workspace, so
`cargo test --workspace` does not need SDL2.

```sh
cd firmware/sim
cargo run
cargo run -- --keys '3/4>+2^3\n' --png screen.png   # type keys, save the screen
```

In `--keys`, `\n` is EXE and `>`/`<` move the cursor.

## Keys

The characters are the same over the board's serial console (`firmware/handheld`).

| Key | PC | Calculator |
|---|---|---|
| `0`–`9`, letters | as typed | digits, variables |
| `,` `.` | either | decimal comma |
| `+` `-` `*` `:` `=` `(` `)` | as typed | operators, `×`, `÷` |
| `/` | | fraction; a number before it becomes the numerator |
| `^` | | exponent |
| `_` | | index (`r_1`) |
| Enter | | EXE |
| Backspace | | DEL |
| Esc, Delete, Ctrl+X | | AC |
| Tab | | MODE: the mode tiles (then 1–5, or the arrows and Enter) |
| F1 then Tab | | SHIFT+MODE: setup (↑/↓ choose a line, ←/→ change it) |
| arrows | | cursor; ↑/↓ outside a fraction step through earlier calculations |
| F1, Ctrl+F | | SHIFT (SHIFT, then √ is the n-th root) |
| F2, Ctrl+K | | ALPHA (only shown for now) |
| F3, Ctrl+D | | S⇔D: the result exact or as a decimal |
| Ctrl+W | | √ |
| Ctrl+Q | | x² |
| Ctrl+P | | π |
| Ctrl+E | | e |
| Ctrl+S, Ctrl+O, Ctrl+T | | sin, cos, tan |
| Ctrl+L, Ctrl+G | | ln, log |
| Ctrl+A | | Ans |

SDL reports only the unshifted key, so the shifted characters in the window follow a German
layout (Shift+7 is `/`, Shift+8 `(`, Shift+0 `=`, Shift++ `*`). The number pad works on any
layout.
