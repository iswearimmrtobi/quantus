---
sidebar_position: 4
title: Keyboard shortcuts
---

# Keyboard shortcuts

Everything except typing is reachable from the keyboard. Press <kbd>F10</kbd> to open the menu.

## Calculation

| Key | Action | Menu |
|---|---|---|
| <kbd>Enter</kbd> or <kbd>Ctrl</kbd>+<kbd>Enter</kbd> | Evaluate | Calculation → Evaluate |
| <kbd>Ctrl</kbd>+<kbd>N</kbd> | New calculation | Calculation → New calculation |
| <kbd>\\</kbd> or <kbd>Ctrl</kbd>+<kbd>L</kbd> | Switch between formula editor and LaTeX source | Calculation → Write LaTeX |
| <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>C</kbd> | Copy the result as LaTeX | Calculation → Copy result |
| <kbd>Ctrl</kbd>+<kbd>Alt</kbd>+<kbd>C</kbd> | Copy the whole calculation as an `align*` block | Calculation → Copy whole calculation |
| <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>K</kbd> | CALC: put values in for the letters of the input | Calculation → CALC |
| <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>S</kbd> | SOLVE the input for a letter | Calculation → SOLVE |
| <kbd>Ctrl</kbd>+<kbd>D</kbd> | Next form of the result (fraction, mixed, recurring, decimal …) | Calculation → Next result form |
| <kbd>Ctrl</kbd>+<kbd>E</kbd> / <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>E</kbd> | ENG / ←ENG: the result in engineering notation, the exponent moved by three | Calculation → ENG, ←ENG |
| <kbd>Ctrl</kbd>+<kbd>Shift</kbd>+<kbd>N</kbd> | New session | Calculation → New session |
| <kbd>Ctrl</kbd>+<kbd>O</kbd> | Sessions panel | Calculation → Sessions |
| | Quit | Calculation → Quit |

## Edit

The **Edit** menu has **Undo**, **Redo**, **Cut**, **Copy**, **Paste** and **Select all** with the
usual keys of your system (<kbd>Ctrl</kbd>+<kbd>Z</kbd>, <kbd>Ctrl</kbd>+<kbd>C</kbd>, …).

## View

| Key | Action |
|---|---|
| <kbd>Ctrl</kbd>+<kbd>J</kbd> | Show or hide the steps |
| <kbd>Ctrl</kbd>+<kbd>,</kbd> | Variables panel |
| <kbd>Ctrl</kbd>+<kbd>H</kbd> | History panel |
| <kbd>Ctrl</kbd>+<kbd>K</kbd> | Library window: constants and unit conversions |
| <kbd>Ctrl</kbd>+<kbd>G</kbd> | Writeup window: the session as given, wanted and solution |
| <kbd>Ctrl</kbd>+<kbd>F</kbd> | Quick search: type part of a constant's, unit's or function's name, <kbd>Enter</kbd> puts the first hit in (<kbd>Alt</kbd>+<kbd>Enter</kbd>: a unit alone) |
| <kbd>Ctrl</kbd>+<kbd>.</kbd> | Settings window |
| <kbd>Esc</kbd> | Leave the LaTeX source, or close the panel or window |
| <kbd>Ctrl</kbd>+<kbd>0</kbd> / <kbd>Ctrl</kbd>+<kbd>+</kbd> / <kbd>Ctrl</kbd>+<kbd>-</kbd> | Reset zoom / zoom in / zoom out |
| <kbd>F11</kbd> | Full screen |

## In the formula editor

| Key | Gives |
|---|---|
| <kbd>/</kbd> | a fraction |
| <kbd>*</kbd> | $\cdot$ |
| <kbd>^</kbd> / <kbd>_</kbd> | a superscript / a subscript |
| <kbd>→</kbd> | leaves the index, exponent or bracket you are in |
| <kbd>\\</kbd> | opens the LaTeX source |
| `▶` or `\blacktriangleright` in the LaTeX source | converts to the unit after it, see [Converting units](../guides/physics/convert.md) |

## In the LaTeX source

| Key | Action |
|---|---|
| <kbd>Enter</kbd> | Evaluate |
| <kbd>Shift</kbd>+<kbd>Enter</kbd> | New line |
| <kbd>Esc</kbd> | Back to the formula editor |

## In the linear system

| Key | Action |
|---|---|
| <kbd>Enter</kbd> | Solve |
| <kbd>Enter</kbd> twice, or <kbd>Shift</kbd>+<kbd>Enter</kbd> in the LaTeX source | New line below |
| <kbd>Backspace</kbd> in an empty line | Remove the line |
| <kbd>↑</kbd> / <kbd>↓</kbd> | Move between the lines |
| <kbd>Tab</kbd> / <kbd>Shift</kbd>+<kbd>Tab</kbd> in the matrix | Next / previous cell, row by row |
| <kbd>Enter</kbd> in the matrix | Solve |

## In the dialogs

| Where | Key | Action |
|---|---|---|
| CALC, SOLVE | <kbd>Enter</kbd> / <kbd>Esc</kbd> | Calculate or solve / close |
| Quick search | <kbd>↑</kbd> <kbd>↓</kbd> or <kbd>Tab</kbd> <kbd>Shift</kbd>+<kbd>Tab</kbd> | Choose a hit |
| Quick search | <kbd>Enter</kbd> / <kbd>Alt</kbd>+<kbd>Enter</kbd> | Put in the hit / a unit without `▶` |
| Library | <kbd>Enter</kbd> in the filter | Put in the first hit (<kbd>Alt</kbd>: a unit alone, <kbd>Shift</kbd>: the window stays open) |
| Library | <kbd>Shift</kbd>+click / <kbd>Alt</kbd>+click | Put in and keep the window open / a unit without `▶` |
| Session name | <kbd>Enter</kbd> / <kbd>Esc</kbd> | Rename / cancel |
| Analysis tasks | <kbd>Enter</kbd> in a field | Run the task |

## In the schematic editor

| Key / mouse | Action |
|---|---|
| <kbd>Space</kbd> or <kbd>R</kbd> | Rotate the selected parts by 90° |
| <kbd>Shift</kbd>+<kbd>R</kbd> | Turn the selected resistors by 45° |
| <kbd>Del</kbd> or <kbd>Backspace</kbd> | Delete the selected parts |
| <kbd>Esc</kbd> | Clear the selection |
| Drag from a terminal, or from a grid point on a wire | Draw a wire |
| Drag on empty canvas | Select several parts |
| Drag with the right or middle button, or scroll | Move the canvas |
| <kbd>Ctrl</kbd>+wheel, or pinch | Zoom |
| <kbd>Home</kbd> | Back to the start view |
| Click a current arrow (Kirchhoff) | Reverse it |

## In the analysis graph

| Mouse | Action |
|---|---|
| Wheel | Zoom |
| Drag | Move |
| Click a point | Select it in the list below the graph |
