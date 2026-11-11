---
sidebar_position: 3.5
title: Theme files
---

# Theme files

A theme is a [TOML](https://toml.io/) file with the extension `.toml`. Its name without `.toml`
is the theme's **id**. The same file works in the desktop app and on the handheld.
[Themes](../basics/themes.md) says where the files go and how to pick one.

The built-in themes are the files in [`themes/`](https://github.com/iswearimmrtobi/quantus/tree/dev/themes)
in the repository: `dark.toml` and `light.toml` give every key, `black.toml` and `paper.toml`
only what they change, and `contrast-dark.toml` and `contrast-light.toml` show colours chosen for
colour blindness. Copying one of them is the quickest way to start.

## A first theme

A theme only gives what it changes. Everything else comes from its **base**, which is
`dark` unless you name another. This file is a complete theme:

```toml
# sunset.toml: the dark theme on a warm brown, with orange accents.
name = { en = "Sunset", de = "Abendrot" }

[colors]
background = "#2a1d17"
surface = "#33241c"
primary = "#f0a070"
accent = "#ffc58f"
```

This one starts from the light theme and gives the handheld's mode tiles its own colours too:

```toml
name = "Paper"
base = "light"

[colors]
background = "#fbf8f1"
surface = "#fffdf8"
primary = "#3f6fb5"
primary-dim = "#dde6f4"

[handheld]
result = "#2f5a9a"
```

## The top of the file

| Key | Value | Default |
|---|---|---|
| `name` | the name in the lists: a text, or a text per language, `{ en = "…", de = "…" }` | the id |
| `base` | the id of the built-in theme to start from: `dark`, `light` or another [built-in one](../basics/themes.md) | `dark` |
| `dark` | `true` for a dark theme, `false` for a light one | the base's |
| `[colors]` | the colours of both programs, below | the base's |
| `[handheld]` | the colours only the handheld has, below | the base's |

- **`name`** in a language the list does not have falls back to English, then to any language
  given. A plain text is the name in every language.
- **`base`** must be a built-in theme, never another file of yours. A file of yours named
  `light.toml` replaces the built-in Light in the list, but `base = "light"` still means the
  built-in one, so such a file can start from the theme it replaces.
- **`dark`** tells the desktop's system parts which way to draw: scroll bars, form controls and
  the text cursor of fields. Quantus does not check it against your colours, so set it to match
  your background.

## Colours

A colour is a text `"#rrggbb"`, or `"#rrggbbaa"` with an opacity from `00` (clear) to `ff`
(solid). Upper and lower case are both fine. The handheld has no opacity: it uses the
`rrggbb` part only, so give colours with an opacity on the handheld as solid ones in
`[handheld]`, or keep them for keys only the desktop uses.

### `[colors]`

The desktop sets each key as the CSS variable of the same name (`--background`, …). The column
*Handheld* says whether the handheld uses it too. The defaults are the Dark theme's.

| Key | What it colours | Handheld | Dark |
|---|---|---|---|
| `background` | the window behind everything; the handheld's screen | yes | `#17161d` |
| `surface` | panels, cards, dialogs, the plot area; the handheld's panels and soft keys | yes | `#1e1d27` |
| `surface-raised` | raised parts: the input row, chips, dropdown entries, the open session | mixed with `surface` for the second row of soft keys | `#262532` |
| `surface-hover` | a button or line under the mouse | – | `#2d2b3a` |
| `hairline` | thin lines between parts, the plot grid | yes | `#322f3f` |
| `hairline-strong` | borders of buttons and fields | – | `#433e56` |
| `text` | the text and the input | yes | `#edebf3` |
| `text-muted` | labels, hints, units under results, asymptotes | yes | `#8b87a3` |
| `text-faint` | placeholders, axis labels, disabled parts | yes | `#55506a` |
| `primary` | the cursor, focus, the chosen tab and segment, function curves | yes: the caret, the menu highlight, the chosen tile | `#9186e8` |
| `primary-dim` | the fill of chosen buttons, the selection behind text | yes: the top bar, the selected line | `#423a63` |
| `accent` | results, the tangent and normal | yes: lit soft keys, the EXAM mark, rings around buttons | `#d8a9ff` |
| `accent-dim` | the fill of highlighted results and accent buttons | yes: the fill of lit soft keys | `#4e3c63` |
| `danger` | error messages, delete buttons | – (see `error` below) | `#ff8fa8` |
| `glow` | the soft light behind a result | – | `#d8a9ff38` |
| `overlay` | the shade behind the quick search for constants and units | – | `#0a090e59` |
| `overlay-strong` | the shade behind dialogs such as CALC and SOLVE | – | `#0a090e99` |
| `point-zero` | zeros of the curve sketch | – | `#7fd1b9` |
| `point-y-intercept` | the y-intercept | – | `#8fb8ff` |
| `point-max` | maxima | – | `#ff8fa8` |
| `point-min` | minima | – | `#9186e8` |
| `point-inflection` | inflection points | – | `#d8a9ff` |
| `point-saddle` | saddle points | – | `#ffd27f` |
| `geometry-0` … `geometry-7` | the objects of the Geometry view, in order; the ninth takes `geometry-0` again | – | `#9186e8` `#7fd1b9` `#ff8fa8` `#ffd27f` `#8fb8ff` `#d8a9ff` `#f0a070` `#b8e07a` |

### `[handheld]`

| Key | What it colours | Dark |
|---|---|---|
| `result` | results, and units and conversions on the soft keys | `#bfa0f2` |
| `error` | error messages | `#c98a96` |
| `shift` | the SHIFT mark in the top bar | `#c9a46a` |
| `alpha` | the ALPHA mark in the top bar | `#8fb3a0` |
| `unit` | the UNIT mark in the top bar | `#9186e8` |
| `tiles` | the mode tiles: fourteen pairs `["tile", "chosen tile"]`, in the order of the mode menu | fourteen muted colours, purple to mauve |

`tiles` is all or nothing: give all fourteen pairs, or leave the key out and keep the base's.

```toml
[handheld]
tiles = [
    ["#4a3d6b", "#63528e"], # purple
    ["#34446b", "#46598c"], # blue
    ["#2f5560", "#3e6f7d"], # teal
    ["#35574a", "#467262"], # green
    ["#4d5536", "#666f47"], # olive
    ["#62513a", "#806a4c"], # amber
    ["#643f35", "#835346"], # rust
    ["#5e3640", "#7b4754"], # red
    ["#5c3756", "#784871"], # plum
    ["#3a3a6e", "#4d4d90"], # indigo
    ["#2f4f6a", "#3e678b"], # steel
    ["#454a5a", "#5c6276"], # slate
    ["#3d523a", "#516c4d"], # moss
    ["#55465e", "#705c7c"], # mauve
]
```

## Errors and warnings

| In the file | What happens |
|---|---|
| broken TOML, a colour that is not `#rrggbb` or `#rrggbbaa`, a `base` that is not built in, `tiles` without fourteen pairs | the theme cannot be chosen; the list says why, with the key: `colors.text: "red" is not a colour, write #rrggbb or #rrggbbaa` |
| a key Quantus does not know, such as a typo `backgrund` | ignored; while the theme is chosen, the desktop lists it under the theme buttons (`colors.backgrund: unknown, ignored`) |
| a theme chosen earlier that is gone or broken now | the app shows Dark and says why; the setting is kept, so the theme comes back once the file is fixed |

## Checking a theme

Look at these before you share a theme. They use the most colours:

- **A calculation with working**: the result with its glow and units, the steps, an error (type
  `1/`).
- **Analysis**: a curve sketch of `x^3-3x` with its points, the tangent and the area.
- **Geometry**: a scene with several objects.
- **Settings**: the chosen and unchosen segments, a dropdown.
- **CALC or SOLVE**: the shade behind the dialog.
- **Handheld**: the mode tiles, a SETUP page with a selected line, the soft keys with SHIFT on,
  and a wallpaper if you use one.

:::note
Two colours in the working come from the engine, not from the theme: the pivot (orange) and the
changed entries (blue) in the steps of a linear system, and a cancelled factor when
**Settings → Steps → Strike out cancelled factors** is off (orange). They are readable on both dark and light
backgrounds.
:::
