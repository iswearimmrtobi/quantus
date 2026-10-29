---
sidebar_position: 1
title: Install
---

# Install

Download the latest release from
[GitHub Releases](https://github.com/iswearimmrtobi/quantus/releases/latest), or build Quantus
yourself as described in [Building](../contributing/building.md).

## Windows

Take one of the two installers:

- `Quantus_0.1.0_x64-setup.exe`, an NSIS installer, or
- `Quantus_0.1.0_x64_en-US.msi`, an `.msi` package.

Both need WebView2, which is part of Windows 10 and 11; if it is missing, the installer
downloads it.

## Linux

| Package | Size | Notes |
|---|---|---|
| `Quantus_0.1.0_amd64.AppImage` | ~100 MB | Ships its own WebKitGTK, runs on most distributions |
| `Quantus_0.1.0_amd64.deb` | ~6 MB | Uses the system's WebKitGTK 4.1 |

Make the AppImage executable and run it:

```sh
chmod +x Quantus_0.1.0_amd64.AppImage
./Quantus_0.1.0_amd64.AppImage
```

Install the `.deb` with `sudo apt install ./Quantus_0.1.0_amd64.deb`.

## Where Quantus keeps its settings

Settings are saved in the app's config directory under the identifier
`dev.tobifrosch.quantus` (on Linux `~/.config/dev.tobifrosch.quantus/`). Delete that folder,
or use **Settings → Restore defaults**, to start over.

Next: [Your first calculation](first-calculation.md).
