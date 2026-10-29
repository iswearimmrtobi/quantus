---
sidebar_position: 6
title: Writing the docs
---

# Writing the docs

This site is built with [Docusaurus](https://docusaurus.io/) from `docs/`.

## Run it

```sh
cd docs
npm install
npm start          # dev server with hot reload at http://localhost:3000/
npm run build      # production build into docs/build/; fails on broken links
```

## Structure

| Folder | For | Reader question |
|---|---|---|
| `docs/docs/getting-started/` | install, first steps, concepts | How do I begin? |
| `docs/docs/guides/` | one task or mode per page | How do I do X? |
| `docs/docs/reference/` | complete lists, mostly tables | What exactly is supported? |
| `docs/docs/contributing/` | code, tests, docs | How does it work, how do I change it? |

The sidebar is generated from the folders. Each folder has a `_category_.json` with its label and
position; each page sets `sidebar_position` and `title` in its front matter. Design documents and
plans are not part of the site; they stay in `design/` and `plans/`.

## Conventions

- **Show input and output.** Put what the user types in a `latex` code block or inline code, and
  what Quantus answers as rendered maths (`$…$`, `$$…$$`, KaTeX). For a calculation with working,
  use a two-column table with the labels SYMBOLIC, SUBSTITUTED, STEP, RESULT.
- **Use real output.** Every result on the site comes from the engine at default settings
  (Standard step detail, precision 50), not from a hand calculation. Say so when a setting
  changes it.
- **Take facts from the code.** Settings come from `settings.rs`, shortcuts from `shell.rs` and
  `App.tsx`, labels from `desktop/src/i18n/en.ts`, units from `units/lookup.rs`.
- **Name UI elements in bold**, exactly as the English interface spells them: **Settings → Step
  detail**. Keys as `<kbd>Ctrl</kbd>+<kbd>J</kbd>`.
- **Guides are short.** A goal, the steps, the result, then *See also* into the reference.
- **Reference pages are tables.** No narrative.
- **Admonitions** (`:::tip`, `:::note`, `:::caution`) for the things that trip people up.
- **Plain voice.** Second person, present tense, short sentences.

The pages are CommonMark, not MDX (`markdown.format: "detect"`), so `<`, `{` and `}` in the text
need no escaping. Inside a table, write `\|` for a `|` in maths.

## When the code changes

Update the page that describes the behaviour in the same commit: a new setting goes into
[Settings](../reference/settings.md), a new command into [Syntax](../reference/syntax.md) and
the guide of its mode, a removed limit out of [Known limits](../reference/limits.md).
