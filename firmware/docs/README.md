# Handheld hardware

Everything about the handheld's hardware: the product, the dev board it is brought up on, and
how to flash and use it. The crates' READMEs cover the software and link here.

- [hardware.md](hardware.md): the product: requirements, components, keypad layout, power, PCB.
- [dev-kit.md](dev-kit.md): the Waveshare ESP32-P4-Module-DEV-KIT: board, 40-pin header,
  onboard GPIO use, the 3.5" display on the header.
- [setup.md](setup.md): toolchain, building, flashing, the serial port, reading the boot log.
- [display-and-touch.md](display-and-touch.md): first start, display colours and orientation,
  the touch controller and its calibration.
- [sd-card.md](sd-card.md): formatting the card and what goes on it.
- [wlan.md](wlan.md): Wi-Fi through the C6 (ESP-Hosted), saved networks, network time.
- [mirror.md](mirror.md): the board as the simulator's screen (testing only).

The crates:

| Crate | What |
|---|---|
| [`ui`](../ui) | The handheld's UI, shared by the simulator and the board (in the main workspace) |
| [`engine`](../engine) | `quantus-core` behind the UI, shared by the simulator and the board (in the main workspace) |
| [`sim`](../sim/README.md) | The UI in a window on the PC, with the real engine |
| [`handheld`](../handheld/README.md) | The firmware for the dev kit: the UI with the engine |
| [`esp32`](../esp32/README.md) | The engine spike: `quantus-core` over serial, no UI |
| [`mirror`](../mirror) | The protocol that sends the simulator's picture to the board |
| [`c6`](../c6/README.md) | The ESP32-C6's firmware (ESP-Hosted, Wi-Fi), which the handheld sends to it |
