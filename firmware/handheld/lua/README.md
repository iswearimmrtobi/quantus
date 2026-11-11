# Lua 5.4.9

The C sources of [Lua](https://www.lua.org) 5.4.9, unchanged, without the stand-alone
interpreter and compiler (`lua.c`, `luac.c`, `onelua.c`). The apps' scripts run in it
(`firmware/apps`, `firmware/docs/apps.md`).

`build.rs` compiles them for the board with the ESP toolchain, and mlua links the library:
lua-src, which builds Lua for mlua on the PC, knows no ESP-IDF target. The version is the one
lua-src 551.0.2 builds on the PC, so the simulator and the tests run the same Lua as the board.
To update, copy the same files from the lua-src release that mlua's lock entry uses.

Lua is under the MIT licence: Copyright © 1994–2026 Lua.org, PUC-Rio (the notice is at the end
of `lua.h`).
