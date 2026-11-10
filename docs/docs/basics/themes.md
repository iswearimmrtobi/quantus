---
sidebar_position: 5
title: Themes
---

# Themes

A theme sets every colour of Quantus: the background, the text, the results, the curves and points
of the graphs, and on the handheld the menus and mode tiles. Quantus comes with two themes:

| Theme | Id | Looks |
|---|---|---|
| **Dark** | `dark` | dark grey with purple accents, the default |
| **Light** | `light` | white and light grey with the same purple accents |

You can add your own: a theme is one small text file, and the same file works in the desktop app
and on the handheld. [Theme files](../reference/themes.md) describes how to write one.

## Desktop app

Open **Settings → General → Theme** and pick one. The change shows at once and is saved with the
other settings.

**Like the system (light or dark)**, the default, shows **Light** or **Dark** as your operating
system is set, and switches when the system does.

### Your own themes

Your themes are the `.toml` files in the `themes` folder of the app's config directory:

| System | Folder |
|---|---|
| Linux | `~/.config/dev.tobifrosch.quantus/themes/` |
| macOS | `~/Library/Application Support/dev.tobifrosch.quantus/themes/` |
| Windows | `%APPDATA%\dev.tobifrosch.quantus\themes\` |

The app makes the folder when it starts. **Open themes folder** opens it in your file manager, and
the hint under the buttons shows the full path.

1. Put a file, say `sunset.toml`, into the folder.
2. Press **Reload** in **Settings → General → Theme**.
3. Pick your theme from the list. Its name comes from the file, or is the file name without
   `.toml` if the file gives none.

While you work on a theme, keep it selected. After each save, press **Reload**, and the
app reads the file again.

A file that cannot be read stays in the list, greyed out, and the reason is shown under the
buttons, for example `sunset.toml: colors.text: "red" is not a colour, write #rrggbb or #rrggbbaa`.
If the chosen theme cannot be read when the app starts, it shows **Dark** and says why in the
settings window. Keys Quantus does not know are ignored; while such a theme is chosen, they are
listed under the buttons.

A file named like a built-in theme (`dark.toml`, `light.toml`) takes the built-in theme's place,
including for **Like the system**.

:::note
On Android the config directory belongs to the app and is not open to other apps, so your own
themes cannot be added there yet.
:::

## Handheld

Open **SETUP → Device → Theme**. The list has the built-in themes first, then the ones on the SD
card. Moving the cursor over a theme shows the whole screen in it, so you see it before you
choose:

| Key | Does |
|---|---|
| <kbd>↑</kbd> <kbd>↓</kbd> | show the theme above or below |
| <kbd>OK</kbd>, <kbd>EXE</kbd>, <kbd>1</kbd>–<kbd>5</kbd> | keep the theme |
| <kbd>←</kbd>, <kbd>DEL</kbd>, <kbd>AC</kbd> | go back to the theme set before |

The theme is kept in the handheld's flash and comes back after a restart. A wallpaper is dimmed
towards the theme's background, so it stays behind the text in a light theme too.

### Your own themes

Copy your `.toml` files into a folder `themes` at the top of the SD card, next to `wallpapers`:

```text
SD card
├── themes/
│   └── sunset.toml
├── wallpapers/
└── sessions/
```

The card is read when the list opens, so a new file shows without a restart. A file that cannot
be read shows as `sunset (!)`; choosing it shows the reason. If the card with the theme you chose
is missing at start, the handheld shows **Dark** until the card is back.

Keep the file names short and in lower case (`sunset.toml`): the name without `.toml` is the
theme's id, and a card without long file names shows it in capitals.

## See also

- [Theme files](../reference/themes.md): every key, with an example to start from
- [Settings](../reference/settings.md): the `theme` key in `settings.json`
