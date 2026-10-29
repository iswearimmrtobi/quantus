---
sidebar_position: 5
title: Translations
---

# Translations

The interface is in English (the default) and German, chosen under **Settings → Language**.
Code, comments and this documentation are in English.

## Where the texts live

| Texts | Where | How |
|---|---|---|
| Interface labels, hints, panel texts | `desktop/src/i18n/en.ts`, `de.ts` | one object per language with the same keys |
| Menu | `desktop/src-tauri/src/shell.rs` | `l("English", "Deutsch")` in `build_menu` |
| Errors of the Tauri shell | `desktop/src-tauri/src/commands.rs` | the `text!` macro |
| Analysis and electronics working, notes and errors | next to their code in `crates/quantus-core/src/analysis/` and `electro/` | the `msg!` macro of `crate::text` |
| Parser and algebra messages | `crates/quantus-core/src/latex/`, `algebra/`, `eval/` | English only |

## The renderer

`en.ts` defines the keys; `de.ts` must have exactly the same ones, which the type
`Record<TextKey, string>` in `desktop/src/i18n/index.tsx` enforces at `npm run typecheck`.
Placeholders are written `{name}` and filled by `t(key, { name })`:

```ts
'footer.counts': '{variables} variables · {history} history entries',
```

To add a text, add the key to `en.ts`, the translation to `de.ts`, and use `t('your.key')` in the
component.

## The engine

Engine texts are written with both languages side by side:

```rust
msg!("Unknown: {}", "Unbekannt: {}", reason.text())
```

`msg!` picks the language of the current call, which the API functions set with a `Scope` from
the settings. Outside a call it is English.

## Adding a language

1. Add the variant to `Language` in `crates/quantus-core/src/settings.rs` and to the `Language`
   type in `desktop/src/api.ts`.
2. Add a dictionary `desktop/src/i18n/xx.ts` and its native name to `LANGUAGE_NAMES`.
3. Extend `msg!`, `text!` and the menu's `l` with the new language. The compiler lists every place
   that needs a text.
