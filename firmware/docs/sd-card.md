# SD card

The dev kit's TF slot is mounted at `/sd` at boot (`firmware/handheld/src/sd.rs`); the serial
monitor prints `SD card mounted` or `no SD card: …`. The card is read only at boot, so insert it
before powering on or press RESET after inserting it.

The slot sits on GPIO39–44, in the domain powered by an LDO that defaults to 1.8 V; the firmware
sets it to 3.3 V and drives GPIO45 low to power the card ([dev-kit.md](dev-kit.md#onboard-gpio-use)).
The bus runs at 10 MHz: at 20 MHz the card reads fine but every write times out
(`sdmmc_write_blocks failed (0x107)`). `QUANTUS_SD_KHZ` at build time picks another clock.

The card is on slot 0 of the SDMMC controller whose slot 1 links the P4 to the C6 for WLAN. It is
mounted through ESP-IDF's `esp_vfs_fat_sdmmc_mount`, which frees only its own slot when a mount
fails, so a missing card leaves WLAN working ([wlan.md](wlan.md#sharing-the-sdmmc-controller-with-the-sd-card)).

## Formatting

The card needs an **MBR** partition table with **one FAT32 partition**. ESP-IDF's FAT driver
reads neither exFAT, which cards over 32 GB come with, nor GPT. Any size works, 128 GB included.
On Linux, with the card at `/dev/sdX` (check with `lsblk`; this erases the card):

```sh
sudo wipefs -a /dev/sdX
echo 'type=c' | sudo sfdisk --label dos /dev/sdX   # one partition over the whole card, type W95 FAT32 (LBA)
sudo mkfs.fat -F 32 -n QUANTUS /dev/sdX1
```

Windows' own format dialog offers only exFAT above 32 GB; use a tool such as *Rufus* or
*guiformat* for FAT32 there.

## What goes on the card

- `wallpapers/`: the pictures for SETUP → Gerät → Hintergrundbild, BMP files of exactly
  480×320 (any BMP colour depth), for example from GIMP or
  `magick in.png -resize 480x320^ -gravity center -extent 480x320 out.bmp`. Long names and
  umlauts are fine. The picture set is kept in flash (NVS) by name and comes back after a
  restart if the card still has it.
- `sessions/`: the sessions of MODE → Sitzungen, one JSON file each (`session-1.json`, …),
  written after every calculation and every change in the menu; `current.txt` names the one
  open, which opens again after a restart. They are the same files as the desktop app's
  `sessions/` in its config directory, so a session can be copied from one to the other (an id
  already in use gets a new one when read). Without a card, sessions are kept until the next
  restart only.

SETUP → Gerät → Speicherplatz shows how full the card is.

## In the simulator

The simulator's folder `firmware/sim/sd` (or the one after `--sd`) stands for the card, with the
same layout. `sd/` is in `.gitignore`.
