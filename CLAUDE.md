# Quantus

A desktop calculator with exact arithmetic, units, equations, integrals and curve sketching.
Rust engine, Tauri 2 shell, React + MathLive frontend.

## Branches

Work on `dev`. `master` only gets a merge at release time, see [RELEASING.md](RELEASING.md).
Nothing runs on pushes to `dev`; the *Check* workflow is started by hand, and runs on the release PR
into `master`. Merging that PR tags and builds the release.

## Layout

- `crates/quantus-core`: the engine, no UI. `eval/`, `electro/`, `linear`, `numeric`, `quantity`,
  `settings`, `session`, `text`, `api`. Tests are integration tests in `tests/`.
- `desktop/`: the Tauri app. Rust shell in `src-tauri/`, frontend in `src/`.
- `firmware/`: the handheld. `ui` (in the workspace) is its UI; `sim`, `handheld` and the engine
  spike `esp32` (the engine over serial, no UI) are their own workspaces; both firmwares target
  the ESP32-P4 dev kit. `c6` is the ESP-IDF (C) firmware of the kit's ESP32-C6 (Wi-Fi through
  ESP-Hosted), which the handheld sends to it. `mirror` (in the workspace, testing only) sends the
  simulator's picture to the board. `engine` (in the workspace) joins `quantus-core` to the UI for
  both. `tools/` holds the keypad PCB scripts. `docs/` holds the hardware documentation (product, dev kit,
  setup, display, SD card, mirror); the crates' READMEs link there.
- `locales/`: every text, one JSON file per language, read by `crates/quantus-texts` (compile time)
  and the frontend.
- `docs/`: Docusaurus site; contributor pages in `docs/docs/contributing/`.
- `design/`: specifications. `plans/plans.md`: open work. `test/`: the generated test set.

## Commands

```sh
cargo test --workspace
cargo clippy --workspace --all-targets
cd desktop && npm run typecheck
cd desktop && npm run dev        # app with hot reload
cd desktop && npm run android:dev  # Android app (needs the SDK and NDK, see building.md)
cargo run --release -p quantus-core --example run_cases   # the generated test set
```

Details: `docs/docs/contributing/building.md` and `testing.md`.

## Conventions

- Add user-visible changes to `## [Unreleased]` in `CHANGELOG.md`.
- Texts go into every file of `locales/` (`cargo test -p quantus-texts` checks them).
- After changing a dependency, regenerate `THIRD-PARTY-LICENSES.md` (`npm run licenses` in `desktop/`).
