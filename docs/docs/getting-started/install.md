---
sidebar_position: 1
title: Install
---

# Install

Download the latest release from
[GitHub Releases](https://github.com/iswearimmrtobi/quantus/releases/latest), or build Quantus
yourself as described in [Building](../contributing/building.md).

## Windows

Run the installer, `Quantus_0.1.0_x64_en-US.msi`. It needs WebView2, which is part of
Windows 10 and 11; if it is missing, the installer downloads it.

## Linux

Quantus comes as an AppImage, `Quantus_0.1.0_amd64.AppImage` (~100 MB). It ships its own
WebKitGTK and runs on most distributions. Make it executable and run it:

```sh
chmod +x Quantus_0.1.0_amd64.AppImage
./Quantus_0.1.0_amd64.AppImage
```

## Where Quantus keeps its settings

Settings are saved in the app's config directory under the identifier
`dev.tobifrosch.quantus` (on Linux `~/.config/dev.tobifrosch.quantus/`). Delete that folder,
or use **Settings → Restore defaults**, to start over.

Next: [Your first calculation](first-calculation.md).
