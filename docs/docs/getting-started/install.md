---
sidebar_position: 1
title: Install
---

# Install

:::note
Quantus has no release yet. Until there is one, you build it yourself or take the Windows
installer from the CI.
:::

## Windows

The *Windows build* workflow on GitHub builds an installer for every `v*` tag, and can be
started by hand from the **Actions** tab. Open a finished run and download the
`quantus-windows` artifact. It contains:

- an NSIS installer (`.exe`), and
- an `.msi` package.

Both need WebView2, which is part of Windows 10 and 11; if it is missing, the installer
downloads it.

## Linux

Build the AppImage or the `.deb` from source as described in [Building](../contributing/building.md):

```sh
cd desktop
npm install
npm run bundle
```

The packages land in `target/release/bundle/`:

| Package | Size | Notes |
|---|---|---|
| AppImage | ~100 MB | Ships its own WebKitGTK, runs on most distributions |
| `.deb` | ~6 MB | Uses the system's WebKitGTK 4.1 |

Run the AppImage directly:

```sh
./target/release/bundle/appimage/Quantus_0.1.0_amd64.AppImage
```

## Where Quantus keeps its settings

Settings are saved in the app's config directory under the identifier
`dev.tobifrosch.quantus` (on Linux `~/.config/dev.tobifrosch.quantus/`). Delete that folder,
or use **Settings → Restore defaults**, to start over.

Next: [Your first calculation](first-calculation.md).
