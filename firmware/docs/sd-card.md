# SD card

The dev kit's TF slot is mounted at `/sd` at boot (`firmware/handheld/src/sd.rs`); the serial
monitor prints `SD card mounted` or `no SD card: …`. The card is read only at boot, so insert it
before powering on or press RESET after inserting it.

The slot sits on GPIO39–44, in the domain powered by an LDO that defaults to 1.8 V; the firmware
sets it to 3.3 V and drives GPIO45 low to power the card ([dev-kit.md](dev-kit.md#onboard-gpio-use)).

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

SETUP → Gerät → Speicherplatz shows how full the card is.

## In the simulator

The simulator's folder `firmware/sim/sd` (or the one after `--sd`) stands for the card, with the
same layout. `sd/` is in `.gitignore`.
