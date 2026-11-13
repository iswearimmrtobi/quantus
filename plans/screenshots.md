# Screenshots on the handheld

## Context

`plans/feature-ideas.md` idea 28: a key combination saves the screen to `/screenshots/` on the
SD card, for questions to a teacher, bug reports, the documentation and the website.

The display cannot be read back (`firmware/handheld/src/display.rs`), but the renderer draws in
bands (`firmware/ui/src/render/renderer.rs`: 480×320, bands of 32 rows, RGB565) and sends each
band to a `DrawTarget`. A screenshot draws the frame once more, with a fresh renderer, into a
target that writes the bands into a file. No frame buffer is needed.

Decisions (from the questions):

| Topic | Decision |
|---|---|
| Keys | **SHIFT + 7** (CAPTURE, as on the fx-CG50), on every screen; PrintScreen on a USB keyboard and in the simulator |
| Format | **BMP, 24 bit**, bottom-up: opens everywhere, and works as a wallpaper |
| Name | `2026-11-12_14-03-27.bmp` when the clock is set, else `screenshot-1.bmp`, the first free number (as the sessions do) |
| Feedback | A short notice over the top bar ("Gespeichert: …", no card, the error); it goes after two seconds or with the next key, and is not in the picture |

Current state, for reference:
- Rendering: `Frame::new(&app)` (`render/mod.rs`) → `Renderer::render(frame, display)`, one
  `fill_contiguous` per band; `Renderer::new()` sends every band.
- Card: the `Backend` trait (`firmware/ui/src/backend.rs`) has the card's reads (wallpapers,
  themes, `storage`), no general write. The board mounts the card at `/sd` (`handheld/src/sd.rs`),
  the simulator uses the `--sd` folder.
- Keys: `layer.rs::shift` is the SHIFT table; SHIFT only latches on the calculation screen and an
  app's typing line (`app.rs::press`). Digits have no SHIFT legend.
- Clock: `clock::now()` is `None` until set (no RTC yet), `clock::local` gives the date.
- No notice/toast exists; menus show messages inside their own pages.

## Step 1 — The plan

- [x] This file, and its pointer in `plans/plans.md` §8.

## Step 2 — A BMP writer for the renderer

- [x] `firmware/ui/src/render/bmp.rs`: the 54-byte header (24 bit, bottom-up, rows padded to
      four bytes) and `Writer<W: Write + Seek>`, a `DrawTarget<Color = Rgb565>` that writes each
      band's rows as BGR at their bottom-up place; `bmp::write(frame, out)` renders with a fresh
      `Renderer` into it.
- [x] Test (`firmware/ui/tests/screenshot.rs`): a frame written into memory, read back with tinybmp, equals the same frame drawn
      into a plain picture, pixel for pixel; the file is 54 + 480·320·3 bytes.

## Step 3 — A notice over the top bar

- [x] `App::notify(text, error)`: a notice drawn over the top bar, gone after two seconds
      (`tick`) or with the next key.
- [x] Test: it shows, and goes.

## Step 4 — SHIFT + 7 saves the screen

- [x] `Key::Screenshot`; SHIFT + 7 in the SHIFT table; PrintScreen on a USB keyboard (`hid.rs`).
- [x] SHIFT latches on every screen: on screens without SHIFT legends only SHIFT + 7 means
      something; any other key drops it and does what it did.
- [x] `Backend::create(path)`: a new file on the card (folder made, never overwritten);
      `Create::{Exists, NoCard, Failed}`; general, so the export of idea 27 can use it.
- [x] `App::screenshot()`: the modifier cleared, the frame written to
      `screenshots/<date or number>.bmp`, then the notice.
- [x] Texts `hui.screenshot.saved` and `hui.screenshot.failed` in every file of `locales/`; no
      card says `hui.no_card`, which exists.
- [x] Tests: on the calculation screen, in a menu and on the mode tiles; the date name and the
      numbers; the SHIFT mark not in the picture; no card; SHIFT then another key in a menu.

## Step 5 — Board and simulator

- [x] Board: `create` under `/sd` (no card → `NoCard`), buffered; a log line on the serial port.
      Both go through `backend::create_in` (folders made, `create_new`, an 8 KB buffer).
- [x] Simulator: `create` under the `--sd` folder; PrintScreen; the key list; CAPTURE on the
      keypad's 7 with SHIFT.

## Step 6 — Docs and wrap-up

- [ ] `CHANGELOG.md`, `firmware/docs/features.md`, `firmware/docs/sd-card.md`, the keys in
      `firmware/handheld/README.md` and `firmware/sim/README.md`, the layer table in
      `design/handheld-menus.md` §7.4.1.
- [ ] Idea 28 out of `plans/feature-ideas.md`.

## Verification

```sh
cargo test --workspace
cargo clippy --workspace --all-targets
cargo test -p quantus-texts
```

Simulator: SHIFT + 7 and PrintScreen on the calculation screen, in a menu and on the graph; the
files in `sd/screenshots/` open in an image viewer, and one copied to `sd/wallpapers/` works as a
wallpaper. Board: SHIFT, 7 over the serial port, the log line, the file on the card.

## Effort

S, as in the idea: about a day.
