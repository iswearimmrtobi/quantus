# ESP32-P4-Module-DEV-KIT

The Waveshare dev board used to bring up the handheld on the ESP32-P4 before our own PCB
([calculator-hardware.md](calculator-hardware.md)). It carries the same ESP32-P4-Module the
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
- TF slot (SDIO 3.0), 100 Mbit Ethernet (RJ45), USB-A OTG 2.0 HS (jumper for host mode).
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
it to 3.3 V (LDO channel 4, see calculator-hardware.md) and drives GPIO45 low to power the card
(`firmware/handheld/src/sd.rs`). These are SDMMC slot 0's fixed IOMUX pins. GPIO24/25 being the USB Serial/JTAG port confirms the
choice for our own PCB.

## The 3.5" RPi display on the header

Because the header has the Pi layout, the display (Waveshare (A) layout, see
[firmware/handheld](../firmware/handheld/README.md#wiring)) plugs on directly or with jumpers pin
for pin. Its signals then land on these P4 GPIOs:

| Display signal | Pi header | P4 GPIO |
|---|---|---|
| SCLK | pin 23 | 0 |
| MOSI | pin 19 | 3 |
| MISO | pin 21 | 2 |
| LCD_CS | pin 24 | 36 |
| TP_CS | pin 26 | 32 |
| LCD_RS | pin 18 | 4 |
| RST | pin 22 | 1 |
| TP_IRQ | pin 11 | 21 |
| 5 V, 3.3 V, GND | pins 2/4, 1/17, 6 … | |

Every pin of the header with what the display connects to it (from the display's (A) pin
table); pins marked – are not connected on the display.

| Display | GPIO | Pin | Pin | GPIO | Display |
|---|---|---:|---:|---|---|
| 3.3 V | 3V3 | 1 | 2 | 5V | 5 V |
| – | 7 | 3 | 4 | 5V | 5 V |
| – | 8 | 5 | 6 | GND | GND |
| – | 23 | 7 | 8 | 37 | – |
| GND | GND | 9 | 10 | 38 | – |
| TP_IRQ | 21 | 11 | 12 | 22 | – |
| – | 20 | 13 | 14 | GND | GND |
| – | 6 | 15 | 16 | 5 | – |
| 3.3 V | 3V3 | 17 | 18 | 4 | LCD_RS |
| MOSI (LCD_SI, TP_SI) | 3 | 19 | 20 | GND | GND |
| MISO (TP_SO) | 2 | 21 | 22 | 1 | RST |
| SCLK (LCD_SCK, TP_SCK) | 0 | 23 | 24 | 36 | LCD_CS |
| GND | GND | 25 | 26 | 32 | TP_CS |
| – | 24 | 27 | 28 | 25 | – |
| – | 33 | 29 | 30 | GND | – |
| – | 26 | 31 | 32 | 54 | – |
| – | 48 | 33 | 34 | GND | – |
| – | 53 | 35 | 36 | 46 | – |
| – | 47 | 37 | 38 | 27 | – |
| – | GND | 39 | 40 | 45 | – |

None of these collide with an onboard part. LCD_CS on GPIO36 is a strapping pin; the display
holds its chip select high when idle, which is the safe level. The display does not use pins 27
and 28, so the **USB** port keeps working with the display plugged on.
