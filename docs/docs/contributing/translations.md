---
sidebar_position: 5
title: Translations
---

# Translations

The interface is in English (the default) and German, chosen under **Settings → Language**.
Code, comments and this documentation are in English.

## One file

Every text of every program is in [`texts/texts.json`](https://github.com/iswearimmrtobi/quantus/blob/dev/texts/texts.json):
the desktop interface, its menu and messages, the engine's working and errors, the handheld's
menus and the names of the constants and units. Each text has a key and one entry per language:

```json
"app.footer.counts": { "en": "{variables} variables · {history} history entries", "de": "{variables} Variablen · {history} Verlaufseinträge" }
```

The first part of the key says who uses it:

| Prefix | Texts | Used as |
|---|---|---|
| `app.` | desktop interface: labels, hints, panels | `t('footer.counts', { variables, history })` in a component, without the prefix |
| `shell.` | desktop menu and the shell's messages (`desktop/src-tauri/src`) | `tr!(language, "shell.menu.evaluate")` |
| `core.` | the engine's working, notes and errors, by module | `msg!("core.latex.parser.input_empty")` |
| `constant.`, `unit.` | names of the constants, units and their categories | `Text(texts!("unit.metre"))` in the tables |
| `hui.`, `engine.` | the handheld's menus and the texts of its engine bridge | `tx!("hui.setup")` in `firmware/ui/src/text.rs` and the pages |

The file also lists the languages, with what differs besides the words:

```json
"languages": {
  "en": { "name": "English", "decimal": ".", "intervals": "round", "quadratic": "abc" },
  "de": { "name": "Deutsch", "decimal": ",", "intervals": "reversed", "quadratic": "pq" }
}
```

`name` is the language's own name in the language picker, `decimal` the decimal separator,
`intervals` `round` for `(0, 1)` or `reversed` for `]0; 1[`, and `quadratic` the formula the
schools teach (`pq` or `abc`), which the **Auto** setting follows.

## How the programs read it

- **Rust** (engine, desktop shell, handheld): the macros of `crates/quantus-texts` read the file
  while compiling. `tr!(language, "key", arguments…)` is a `format!` in that language, and
  `texts!("key")` gives every language for a `const`. Nothing reads JSON at run time, so the
  handheld keeps only the strings themselves. An unknown key is a compile error, and so is a
  translation whose `{}` placeholders do not match the arguments.
- **Engine**: `msg!("key", …)` takes the language of the current call. The API functions set
  it with a `text::Scope` from the settings; outside a call it is English.
- **Desktop frontend**: `desktop/src/i18n/index.tsx` imports the file. `TextKey` is the set of
  `app.` keys, so `npm run typecheck` rejects a key that does not exist.

Rust texts are `format!` strings, so a literal brace is doubled: `{{` and `}}`.
`app.` texts are plain, with `{name}` placeholders that `t` fills.

## Adding a text

Add the key with every language to `texts/texts.json` and use it in the code. Keep the file
sorted by key.

## Checks

`cargo test -p quantus-texts` checks the file:

- every text has every language (a missing one shows the English text meanwhile),
- every language has the same placeholders as English,
- every text is used somewhere in the code.

## Adding a language

1. Add it to `languages` in `texts/texts.json`, with its code as the key (`fr`, `pt-br`).
2. Add a text for it to every entry. Until one is done, the English text stands in, and the
   test lists what is still missing.

That is all: the `Language` enum, the language pickers on the desktop and the handheld and the
settings files all follow the file.
