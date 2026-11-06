---
sidebar_position: 5
title: Translations
---

# Translations

The interface is in English (the default) and German, chosen under **Settings → Language**.
Code, comments and this documentation are in English.

## One file per language

Every text of every program is in [`locales/`](https://github.com/iswearimmrtobi/quantus/tree/dev/locales),
one file per language: `en.json`, `de.json`. They hold the desktop interface, its menu and
messages, the engine's working and errors, the handheld's menus and the names of the constants
and units. Each file maps the same keys to its texts:

```json
"app.footer.counts": "{variables} Variablen · {history} Verlaufseinträge"
```

The first part of the key says who uses it:

| Prefix | Texts | Used as |
|---|---|---|
| `app.` | desktop interface: labels, hints, panels | `t('footer.counts', { variables, history })` in a component, without the prefix |
| `shell.` | desktop menu and the shell's messages (`desktop/src-tauri/src`) | `tr!(language, "shell.menu.evaluate")` |
| `core.` | the engine's working, notes and errors, by module | `msg!("core.latex.parser.input_empty")` |
| `constant.`, `unit.` | names of the constants, units and their categories | `Text(texts!("unit.metre"))` in the tables |
| `hui.`, `engine.` | the handheld's menus and the texts of its engine bridge | `tx!("hui.setup")` in `firmware/ui/src/text.rs` and the pages |

Each file starts with what differs besides the words:

```json
"language": { "name": "Deutsch", "decimal": ",", "intervals": "reversed", "quadratic": "pq" }
```

`name` is the language's own name in the language picker, `decimal` the decimal separator,
`intervals` `round` for `(0, 1)` or `reversed` for `]0; 1[`, and `quadratic` the formula the
schools teach (`pq` or `abc`), which the **Auto** setting follows.

## How the programs read them

- **Rust** (engine, desktop shell, handheld): the macros of `crates/quantus-texts` read the files
  while compiling. `tr!(language, "key", arguments…)` is a `format!` in that language, and
  `texts!("key")` gives every language for a `const`. Nothing reads JSON at run time, so the
  handheld keeps only the strings themselves. An unknown key is a compile error, and so is a
  translation whose `{}` placeholders do not match the arguments.
- **Engine**: `msg!("key", …)` takes the language of the current call. The API functions set
  it with a `text::Scope` from the settings; outside a call it is English.
- **Desktop frontend**: `desktop/src/i18n/index.tsx` imports the files. `TextKey` is the set of
  `app.` keys of `en.json`, so `npm run typecheck` rejects a key that does not exist.

Rust texts are `format!` strings, so a literal brace is doubled: `{{` and `}}`.
`app.` texts are plain, with `{name}` placeholders that `t` fills.

## Adding a text

Add the key to every file of `locales/` and use it in the code. Keep the files sorted by key.

Keys say what the text is for, not what it says: `core.matrix.singular`, not a slug of the
sentence. Within a prefix the module comes next (`core.latex.parser.`), and a group of related
texts gets its own part (`hui.tree.zone.berlin`).

## Checks

`cargo test -p quantus-texts` checks the files:

- every other language has every text of `en.json` and none it lacks (a missing one shows the
  English text meanwhile),
- every language has the same placeholders as English,
- every text is used somewhere in the code.

## Adding a language

1. Copy `locales/en.json` to `locales/xx.json`, named by the language's code (`fr`, `pt-br`), and
   set its `language` details.
2. Translate the texts. Until one is done, delete it or leave it English; the test lists what
   is still missing.

That is all: the `Language` enum, the language pickers on the desktop and the handheld and the
settings files all follow the folder. English comes first, the others by code.
