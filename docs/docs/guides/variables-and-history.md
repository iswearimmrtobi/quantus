---
sidebar_position: 10
title: Variables and history
---

# Variables and history

The side panels keep track of the session: the **Variables** you defined, the **History** of
what you calculated and the **Sessions** themselves. Open them from the rail at the side, or with
the keyboard:

| Key | Panel |
|---|---|
| <kbd>Ctrl</kbd>+<kbd>,</kbd> | Variables |
| <kbd>Ctrl</kbd>+<kbd>H</kbd> | History |
| <kbd>Ctrl</kbd>+<kbd>O</kbd> | Sessions |
| <kbd>Esc</kbd> | close the panel |

The status bar shows the name of the open session and how many variables and history entries
there are; click the name to open the session panel.

## Variables

An input whose label has an index defines a variable:

```latex
r_1 = 0.5m
```

The panel lists each variable with its value. In it you can:

- **click a variable** to insert its name at the cursor, and
- click **✕** (*Delete variable*) to forget it.

Defining the same name again replaces the value. Every later calculation that uses the variable
puts in its value, which you see in the **SUBSTITUTED** line.

A variable can hold any result: a quantity with a unit, a plain number or, in Imaginary mode, a
complex number (`z_1 = 3+4i`).

`\mathrm{Ans}` is the last numeric result and `\mathrm{PreAns}` the one before.

## CALC and SOLVE

Right-click the input (or use the *Calculation* menu) for two tools known from the Casio:

- **CALC** (<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>K</kbd>) asks for a value for every letter of
  the input that has none, then evaluates it. A field left empty takes the session's value.
- **SOLVE** (<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>S</kbd>) solves the input for the letter you
  choose: in Algebra mode `F = m a` for `a`, with values for the others. Exact where the engine
  can, otherwise with Newton's method from the optional start value.

The values you type become session variables.

## History

Every evaluated input goes to the top of the history, with the time and its result; the last
200 are kept. Click an entry to put its input back into the editor, change it and evaluate again.
**Clear history** at the end of the list empties it.

Changing a setting recalculates the current input without adding a history entry.

## Sessions

A session keeps its variables, its history and the system of the Linear systems mode. Each is
saved as a file of its own in `sessions/` in the app's config directory, next to
`settings.json`, after every change, and the session used last opens again on the next start.
The settings and the mode are not part of a session; they stay the same for all.

In the session panel: **New session** (<kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>N</kbd>, named after
the time, "Session 2026-11-01 14:30"), click a session to open it, ✎ renames it, ⧉ duplicates it
and ✕ deletes it after asking. The files are the same as on the handheld.

## Start over

<kbd>Ctrl</kbd>+<kbd>N</kbd> (*New calculation*) clears the input and the result. Variables and
history stay; for a clean slate, start a new session.

## See also

- [Concepts](../getting-started/concepts.md#variables-carry-an-index): which names are variables
