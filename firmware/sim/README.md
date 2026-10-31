# Quantus handheld simulator

The handheld's UI (`firmware/ui`) in a window on the PC, at twice the display's 480×320. It uses
the same band renderer as the board, so the window shows what the display will show. There is no
engine yet: the stub answers with the input itself, and an input with a `?` gives an error.

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
| Tab | | MODE (then 1–5 or ↑/↓ and Enter) |
| arrows | | cursor; ↑/↓ outside a fraction recall earlier inputs |
| F1, Ctrl+F | | SHIFT (SHIFT, then √ is the n-th root) |
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
