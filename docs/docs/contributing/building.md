---
sidebar_position: 2
title: Building
---

# Building

## Prerequisites

| | |
|---|---|
| **Rust** | stable, edition 2021 |
| **Node** | 20 or newer |
| **Linux** | WebKitGTK 4.1 and the usual GTK development packages, see the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) |
| **Windows** | the MSVC build tools and WebView2 (built into Windows 10 and 11) |

## Commands

```sh
cargo test --workspace   # engine tests, golden files and Tauri IPC tests

cd desktop
npm install              # once; also copies the MathLive fonts to public/fonts
npm run dev              # Vite dev server + Tauri window with hot reload
npm run dev:renderer     # only the renderer in a browser, without the Tauri shell
npm run typecheck        # check the TypeScript
npm run bundle           # build the packages
npm run bundle:appimage  # only the AppImage (Linux)
npm run bundle:msi       # only the .msi (Windows)
```

:::tip
`npm install` has to run in `desktop/`, not in the repository root.
:::

The number crates are compiled with `opt-level = 3` even in debug builds (see the root
`Cargo.toml`); without that, integrals and the golden tests are several times slower.

## Packaging

`npm run bundle` runs `tauri build` and writes to `target/release/bundle/`:

| Platform | Output |
|---|---|
| Linux | AppImage (~100 MB, ships WebKitGTK) and `.deb` (~6 MB, uses the system's WebKitGTK) |
| Windows | NSIS installer (`bundle/nsis/*.exe`) and `.msi` (`bundle/msi/`) |

The desktop entry comes from `desktop/src-tauri/packaging/quantus.desktop`. The icons are
generated from `desktop/build/icon.svg` with `npx tauri icon`.

Every package ships `LICENSE` and `THIRD-PARTY-LICENSES.md`, the license texts of all crates and
npm packages in the app. After adding or updating a dependency, regenerate the second one with
`npm run licenses` in `desktop/` (it needs `cargo install cargo-about --features cli`) and commit
it. The same command writes `firmware/THIRD-PARTY-LICENSES.md` for the handheld's firmware: its
crates for the board (`firmware/about.toml`), then its fonts and ESP-IDF
(`firmware/licenses/fonts.md`, kept by hand).

## Android

The same app builds for Android through Tauri's mobile support. The shell is a library
(`src-tauri/src/lib.rs`, with `tauri::mobile_entry_point`); `main.rs` only starts it on the
desktop. The menu, the single-instance plugin and the GTK error dialog exist only on the desktop;
on Android the UI's action bar stands in for the menu, and circuit files go through
`tauri-plugin-fs`, because the file dialog returns `content://` URIs there.

Prerequisites, once:

- JDK 17, the Android SDK (a platform, build tools, platform tools) and the NDK; set `JAVA_HOME`,
  `ANDROID_HOME` and `NDK_HOME`. See the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/#android).
- `rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android`
- An emulator, or a phone with USB debugging on.

```sh
cd desktop
npm run android:dev      # on the emulator or phone, with hot reload
npm run android:build    # APKs under src-tauri/gen/android/app/build/outputs/apk/
```

The Android project in `src-tauri/gen/android/` comes from `npx tauri android init`. The phone
layout is plain CSS (`@media (max-width: 700px)` in `styles.css`), so `npm run dev` in a narrow
window shows it too.

## Release builds in CI

The *Release build* workflow (`.github/workflows/release.yml`) runs the desktop tests and builds
the AppImage on Ubuntu 22.04 and the `.msi` on Windows, by hand from the **Actions** tab or for a
release. The packages are attached to the run as the `quantus-linux` and `quantus-windows`
artifacts. For a release they also go into a draft GitHub release, which is published by hand.

A release starts with a pull request from `dev` into `master`: the *Check* workflow
(`ci.yml`) runs on it and builds both packages, to be tried before the merge. Merging it starts
*Release* (`release-on-merge.yml`), which tags the merge commit with the version from
`Cargo.toml` and calls *Release build*. The steps are in `RELEASING.md` in the repository.

The AppImage is built on the oldest supported Ubuntu runner on purpose: it runs only on systems
whose glibc is at least as new as the one it was built against.

## Next

- [Testing](testing.md): what the tests cover and how to run a part of them
