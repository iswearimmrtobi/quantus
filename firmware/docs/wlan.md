# WLAN

The ESP32-P4 has no radio. The dev kit's ESP32-C6 does Wi-Fi for it, over SDIO with
[ESP-Hosted](https://github.com/espressif/esp-hosted-mcu). `esp_wifi_remote` gives the P4 the usual
`esp_wifi` API on top of that, so esp-idf-svc's `EspWifi` works as on a chip with its own radio.
Both are components from the ESP component registry (`[package.metadata.esp-idf-sys]` in
`firmware/handheld/Cargo.toml`), resolved into `firmware/handheld/components_esp32p4.lock`.

## The link to the C6

| Signal | P4 GPIO |
|---|---|
| SDIO CLK | 18 |
| SDIO CMD | 19 |
| SDIO D0–D3 | 14–17 |
| C6 reset (C6_EN) | 54 |

These are ESP-Hosted's defaults for a P4 with a C6, which the dev kit follows. The settings are
set out in `sdkconfig.defaults`. The C6 uses SDMMC slot 1 at 40 MHz, 4 bits wide.

ESP-Hosted starts before `main`: it resets the C6 through GPIO54 and brings up the link. The boot
log shows this as `eh_sdio: transport[host]: SDIO 4-bit …` and `slave chip id: 0x0d (esp32c6)`.

### Sharing the SDMMC controller with the SD card

The SD card is on slot 0 of the same controller. ESP-IDF supports this:
- `sdmmc_host_init` skips a host that is already up. The log says `SDMMC host already
  initialized, skipping init flow` when the card mounts.
- Each slot is freed on its own.

`src/sd.rs` therefore mounts the card through ESP-IDF's `esp_vfs_fat_sdmmc_mount`, with the per-slot
`sdmmc_host_deinit_slot`. It does not use esp-idf-hal's SDMMC driver: when that driver is dropped
after a failed mount (no card, or not FAT), it shuts down the whole controller, and WLAN with it.

## The C6's firmware

The kit's C6 came with an older ESP-Hosted firmware that reports version 0.0.0. With host 3.0.10
the log shows this error:

```
E (2640) eh_init_evt: major version mismatch — OTA coprocessor from host
```

Scanning and connecting still work with it (tried on 2026-11-02). If something fails that the log
puts down to the C6, update the C6's firmware to the same ESP-Hosted version. Either:
- ESP-Hosted's co-processor OTA from the P4 (`esp_hosted_slave_ota`), or
- flash the C6 directly through its UART header (dev-kit.md).

The firmware does neither yet.

## What the firmware does

`src/wlan.rs` runs WLAN on a thread of its own. The UI reads its state and never waits for it.

- **On/off**: SETUP → Verbindungen → WLAN, kept with the settings. The exam mode switches WLAN off
  and keeps it off. Off stops the Wi-Fi driver; the C6 itself stays powered.
- **Netzwerke**: a scan, about 2.5 s, then the networks in reach, strongest first.
  - A saved network or an open one connects at once.
  - A new secured one asks for its password first.
- **Passwords**: typed on a USB keyboard or over serial. The keypad's ALPHA does not type letters
  yet. AltGr types `@`, `{`, `[`, `]`, `}`, `\`, `~` and `|`.
- **Saving**: a network is saved once the connection stands with its password. At most 8 are
  saved, in NVS (namespace `wlan`, in clear like ESP-IDF's own Wi-Fi settings).
  - While WLAN is on and not connected, the strongest saved network in reach is joined, and tried
    again every 10 s.
  - With none saved, nothing is scanned unless asked.
- **Status**: the network, the address and the signal. The fan in the top bar is dim until
  connected.
- **Network time**: SNTP (`pool.ntp.org`) runs while connected and SETUP → Verbindungen → Uhrzeit →
  Quelle is *Netzwerk*. It sets the system clock. The firmware keeps UTC and shows the time in the
  zone set there (`firmware/ui/src/clock.rs`).
- **Setting the time by hand** sets the system clock too, and the source to *von Hand*. The time
  is lost when the power goes; the product gets an RTC for that ([hardware.md](hardware.md)).

The serial log says what WLAN does: `WLAN: Searching`, `WLAN: 10 networks in 2527 ms`,
`WLAN: Connected { ssid: …, ip: …, rssi: … }`, `WLAN: clock set from the network`.

## Troubleshooting

- **`no Wi-Fi driver`** at boot: the link to the C6 did not come up. Look for `eh_sdio` errors
  further up in the log, and see *The C6's firmware*.
- **`E system_api: 0 mac type is incorrect`** at the first start of Wi-Fi: harmless. The P4 has no
  Wi-Fi MAC of its own; the C6's is used.
- **A network is refused** (`Keine Verbindung mit …`): the password is wrong, or the network uses
  something other than WPA2/WPA3 Personal (enterprise networks are not supported).
- **The time does not come**: a network without internet access, or one that blocks NTP
  (some hotspots with a login page). Set it by hand.
