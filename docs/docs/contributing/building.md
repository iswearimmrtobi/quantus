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
it.

## Windows builds without Windows

The *Windows build* workflow (`.github/workflows/windows.yml`) runs the desktop tests and builds
both installers on every `v*` tag, or by hand from the **Actions** tab. The installers are
attached to the run as the `quantus-windows` artifact.

## Next

- [Testing](testing.md): what the tests cover and how to run a part of them
