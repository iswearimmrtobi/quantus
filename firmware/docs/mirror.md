# Mirror (testing only)

With `--mirror <port>` the board is the simulator's screen: the UI and the engine run on the PC,
you type into the window, and the part of each band that changed goes over serial to the board,
so the display shows exactly what the window shows. Flash the board's `mirror` binary first and
close the serial monitor, since the port takes only one user ([setup.md](setup.md#serial-port-on-linux)):

```sh
cd firmware/handheld && cargo run --release --bin mirror   # flash, then quit the monitor
cd firmware/sim && cargo run -- --mirror /dev/ttyACM0
```

`cargo run --release` in `firmware/handheld` flashes the normal firmware again.

The link runs at 921600 baud, about 90 KB/s, unless the board's `mirror` was built with
`QUANTUS_MIRROR_BAUD=2000000` (then pass `--baud 2000000` to the simulator too). A full screen of
the plain UI is a few KB; with a wallpaper it is about 300 KB (2 bytes a pixel), so about 3 s,
while a caret blink or a key is a small rectangle. The real firmware draws to the display
directly and is not slowed by this. When the board resets it says `READY` and gets the whole
screen again. The protocol is in `firmware/mirror`.
