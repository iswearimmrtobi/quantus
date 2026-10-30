---
sidebar_position: 10
title: Variables and history
---

# Variables and history

The side panels keep track of the session: the **Variables** you defined and the **History** of
what you calculated. Open them from the rail at the side, or with the keyboard:

| Key | Panel |
|---|---|
| <kbd>Ctrl</kbd>+<kbd>,</kbd> | Variables |
| <kbd>Ctrl</kbd>+<kbd>H</kbd> | History |
| <kbd>Esc</kbd> | close the panel |

The status bar shows how many variables and history entries there are.

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

## History

Every evaluated input goes to the top of the history, with the time and its result; the last
50 are kept. Click an entry to put its input back into the editor, change it and evaluate again.

Changing a setting recalculates the current input without adding a history entry.

:::note
Variables and history live as long as the app runs. They are not saved when you close it.
:::

## Start over

<kbd>Ctrl</kbd>+<kbd>N</kbd> (*New calculation*) clears the input and the result. Variables and
history stay.

## See also

- [Concepts](../getting-started/concepts.md#variables-carry-an-index): which names are variables
