# ESP32-P4-Module-DEV-KIT

The Waveshare dev board used to bring up the handheld on the ESP32-P4 before our own PCB
([hardware.md](hardware.md)). It carries the same ESP32-P4-Module the
product uses. Copied from the official guide (as of 2026-11), so it need not be looked up again:
<https://docs.waveshare.com/ESP32-P4-Module-DEV-KIT>. The schematic is linked from its
*Resources* page.

## Board

- ESP32-P4-Module: ESP32-P4NRW32X (2 × RISC-V up to 400 MHz, 768 KB L2MEM, 32 MB PSRAM in the
  package), 16 MB NOR flash, ESP32-C6 for Wi-Fi 6 / BT 5 over SDIO.
- Two USB-C ports, silkscreened **USB TO UART** (UART0, GPIO37/38, through a USB-UART bridge)
  and **USB** (the P4's own USB). Both power the board and can flash it.
- 40-pin header in the Raspberry Pi layout, 28 free GPIOs, 3.3 V logic.
- MIPI-DSI (2-lane, for Waveshare's 5–10.1" DSI screens) and MIPI-CSI (2-lane) connectors.
- TF slot (SDIO 3.0), 100 Mbit Ethernet (RJ45), USB-A OTG 2.0 HS (jumper for host mode; the
  firmware takes a keyboard there, see [below](#usb-keyboard)).
- Audio codec with amplifier, onboard microphone, speaker header (MX1.25 2P, 8 Ω 2 W).
- I²C and I³C connectors, ESP32-C6 UART header, 5 V input (SH1.0 2P), RTC battery
  (rechargeable only), PoE module socket.
- BOOT (hold while powering on or resetting for download mode) and RESET buttons. Only LED: power.

## 40-pin header

Pin 1 is the 3V3 pin at the corner nearest the C6 UART header; odd pins are on the inner row.

| Use | GPIO | Pin | Pin | GPIO | Use |
|---|---|---:|---:|---|---|
| | 3V3 | 1 | 2 | 5V | |
| I²C SDA (shared) | 7 | 3 | 4 | 5V | |
| I²C SCL (shared) | 8 | 5 | 6 | GND | |
| | 23 | 7 | 8 | 37 | UART0 TX (USB TO UART) |
| | GND | 9 | 10 | 38 | UART0 RX (USB TO UART) |
| | 21 | 11 | 12 | 22 | |
| | 20 | 13 | 14 | GND | |
| | 6 | 15 | 16 | 5 | |
| | 3V3 | 17 | 18 | 4 | |
| | 3 | 19 | 20 | GND | |
| | 2 | 21 | 22 | 1 | |
| | 0 | 23 | 24 | 36 | strapping pin |
| | GND | 25 | 26 | 32 | |
| USB D− (USB port) | 24 | 27 | 28 | 25 | USB D+ (USB port) |
| | 33 | 29 | 30 | GND | |
| | 26 | 31 | 32 | 54 | C6_EN (Wi-Fi chip enable) |
| | 48 | 33 | 34 | GND | |
| Audio PA_CTRL | 53 | 35 | 36 | 46 | |
| | 47 | 37 | 38 | 27 | |
| | GND | 39 | 40 | 45 | SD card PWR_CTRL |

The *Use* column notes what is also wired to that GPIO on the board (from the GPIO table
below). Leave those pins alone unless the onboard part is not needed.

## Onboard GPIO use

| GPIO | Use |
|---|---|
| 0–5 | free (header) |
| 6 | C6_GPIO2 (also header pin 15) |
| 7, 8 | I²C SDA, SCL: audio codec, DSI and CSI connectors, I²C connector |
| 9–13 | I²S to the codec: DOUT, LRCK, DIN, BCLK, MCLK |
| 14–17 | C6 SDIO D0–D3 |
| 18, 19 | C6 SDIO CLK, CMD |
| 20–23 | free (header) |
| 24, 25 | USB1P1 N, P: the **USB** port (USB Serial/JTAG) |
| 26, 27 | free (header) |
| 28–31 | Ethernet RXDV, RXD0, RXD1, MDC |
| 32, 33 | free (header) |
| 34 | Ethernet TXD0 |
| 35 | Ethernet TXD1, BOOT button |
| 36 | free (header), strapping pin |
| 37, 38 | UART0 TX, RX: the **USB TO UART** port |
| 39–44 | SD card D0–D3, CLK, CMD |
| 45 | SD card power control |
| 46–48 | free (header) |
| 49–52 | Ethernet TXEN, REF_CLK, PHY_RST, MDIO |
| 53 | audio amplifier enable (PA_CTRL) |
| 54 | C6_EN |
| CHIP_PU | RESET button |

The SD card sits on GPIO39–48, the domain powered by an LDO that defaults to 1.8 V; firmware sets
it to 3.3 V (LDO channel 4, see [hardware.md](hardware.md) §3; [sd-card.md](sd-card.md)) and drives GPIO45 low to power the card
(`firmware/handheld/src/sd.rs`). These are SDMMC slot 0's fixed IOMUX pins. GPIO24/25 being the USB Serial/JTAG port confirms the
choice for our own PCB.

## The 3.5" capacitive touch display on the header

The Waveshare [3.5inch Capacitive Touch LCD](https://www.waveshare.com/wiki/3.5inch_Capacitive_Touch_LCD)
(ST7796S on 4-wire SPI, FT6336U touch on I²C, 320×480, 3.3 V or 5 V supply) has no Pi-layout
plug, only a 15-pin GH1.25 connector (or an 18-pin FPC), so it is wired with jumpers. The 15-pin
numbering is the connector's pin 1 first; check it against the silkscreen. The firmware sets the
pins in `open_display!` in `firmware/handheld/src/display.rs`:

| Display (15-pin) | Signal | Header pin | P4 GPIO |
|---|---|---:|---|
| 1 | VCC | 1 (3V3), **not** 5 V | |
| 2 | 3V3 | 17 (3V3) | |
| 3 | GND | 6 (GND) | |
| 4 | MISO | not connected | the display is never read |
| 5 | MOSI | 19 | 3 |
| 6 | SCLK | 23 | 0 |
| 7 | SD_CS | 17 (3V3) | the TF slot on the display is unused; high keeps it off the bus |
| 8 | LCD_CS | 24 | 36 |
| 9 | LCD_DC | 18 | 4 |
| 10 | LCD_RST | 22 | 1 |
| 11 | LCD_BL | 16 | 5 (high: on) |
| 12 | TP_SDA | 3 | 7 (shared I²C) |
| 13 | TP_SCL | 5 | 8 (shared I²C) |
| 14 | TP_INT | not connected | polled instead |
| 15 | TP_RST | 11 | 21 |

**Supply VCC from 3.3 V, not 5 V.** With VCC on 5 V the touch controller did not answer on this
dev kit, and the display went dark a few seconds after boot; on 3.3 V both work. The module's 3V3
pin (pin 2) was not tested on its own: it is wired to 3V3 in the table, and the touch worked
with it connected.

The touch controller (address 0x38) sits on the board's I²C bus with the audio codec, which has
a different address. The bus needs pull-ups on SDA and SCL (the board should have them for the codec; not checked,
and the display module may have its own); the firmware runs it at 400 kHz. If the picture stays white or black, check LCD_BL and LCD_RST first; if touch does not
answer, `QUANTUS_TOUCH_DEBUG=1` prints the controller's registers.

None of these collide with an onboard part. LCD_CS on GPIO36 is a strapping pin; the display
holds its chip select high when idle, which is the safe level. The display does not use pins 27
and 28, so the **USB** port keeps working with the display wired up.

The earlier prototype display, an RPi display with an ILI9486 and an XPT2046 on the Pi-layout
header, and the classic ESP32 before that, are in git history.

Header pins the display wiring uses (others stay free):

| Use | GPIO | Pin | Pin | GPIO | Use |
|---|---|---:|---:|---|---|
| 3V3 (VCC) | 3V3 | 1 | 2 | 5V | |
| TP_SDA | 7 | 3 | 4 | 5V | |
| TP_SCL | 8 | 5 | 6 | GND | GND |
| | 23 | 7 | 8 | 37 | |
| | GND | 9 | 10 | 38 | |
| TP_RST | 21 | 11 | 12 | 22 | |
| | 20 | 13 | 14 | GND | |
| | 6 | 15 | 16 | 5 | LCD_BL |
| 3V3 (3V3, SD_CS) | 3V3 | 17 | 18 | 4 | LCD_DC |
| MOSI | 3 | 19 | 20 | GND | |
| | 2 | 21 | 22 | 1 | LCD_RST |
| SCLK | 0 | 23 | 24 | 36 | LCD_CS |

## USB keyboard

The USB-A port is the P4's high-speed USB OTG controller on its own pins (no GPIO), which the
firmware runs as a USB host (ESP-IDF's USB Host Library, `firmware/handheld/src/keyboard.rs`).
A keyboard plugged in there types into the UI; the keys are in the handheld's
[README](../handheld/README.md#keys). The serial log shows `USB keyboard on interface …` when it
is recognised. Waveshare's guide names a jumper for host mode; on our board a keyboard works with
the jumpers as they came.
