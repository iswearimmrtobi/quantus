---
sidebar_position: 1
title: Writing apps
---

# Writing apps

An app is one text file in Rhai, a small scripting language that reads like Rust or JavaScript.
Copy it into the folder `/apps` on the handheld's SD card and it appears under **MODE → Apps**,
beside the built-in games. There is no toolchain and nothing to flash.

Apps are for games and for your own calculation pages: a form that asks for the values of one
task and works the result out. In **Prüfungsmodus** no app opens.

## A first app

```rhai
// name: Counter
// name.de: Zähler
// kind: tool
// api: 1
// author: Ada

fn init() { this.n = 0; }

fn key(k) {
    if k == "up"   { this.n += 1; }
    if k == "down" { this.n -= 1; }
}

fn draw() {
    clear("background");
    text(20, 40, `Count: ${this.n}`, "text");
}
```

Save it as `/apps/counter.rhai`. The file's name without `.rhai` is the app's id: letters,
digits, `-` and `_` only.

The app's state is one map, which every function gets as `this`. Functions do not see
variables outside themselves, so keep everything in `this`.

## The header

The comment lines at the top, before the first line of code, describe the app. The list reads
them without running it.

| Line | |
|---|---|
| `name` | What the list shows. Required: a file without it is left out |
| `name.de`, `name.en` | The name in one language |
| `kind` | `game` or `tool`; the list groups by it |
| `api` | The version of this page the app was written for, `1`. An app that needs a newer one is listed in grey |
| `author` | Repeat it for several authors. Without one, **Info** says *Author unknown* |
| `version`, `about`, `about.de`, `license` | Shown by **OPTN → Info** inside the app |

Each value is one line of at most 80 characters.

## Callbacks

Define the functions you need; all are optional. A script without `draw` is only a form
(see [Forms](#forms)).

| Function | Called |
|---|---|
| `init()` | Once, when the app opens |
| `key(k)` | For a key press; `k` is its name |
| `tick(ms)` | On the timer set with `timer`; `ms` since the app opened |
| `draw()` | After every other callback, to describe the whole area |
| `tap(x, y)` | For a tap inside the area |
| `entered(value)` | When an input template gives a value |
| `pick(col, row)` | When a cell of the grid template is picked |
| `leave()` | When the app closes: the place to `save` |

## Keys

`key` gets `"up"`, `"down"`, `"left"`, `"right"`, `"ok"`, `"exe"`, `"del"`, `"0"` to `"9"`,
`"+"`, `"-"`, `"*"`, `"/"`, `"="`, `"."` and `"soft1"` to `"soft12"`. **AC** always leaves the
app, **MODE** leaves to the mode menu, **OPTN** opens the app's menu (**Info**, **Restart**,
**Delete data**, **Quit**). SHIFT, ALPHA and UNIT only work inside an input line or a line of
text. A swipe on the screen comes as an arrow.

## The screen

The top bar shows the app's name and cannot be changed. Below it is the app's area: 480 × 294
pixels, 480 × 250 while the soft keys show, lower again while an input line shows. Lay your app
out for `width()` and `height()`; never assume a size. Coordinates start at the area's top left.

| Function | |
|---|---|
| `width()`, `height()` | The area's size now |
| `clear(color)` | Fills the area |
| `rect(x, y, w, h, color)` | A filled rectangle |
| `round(x, y, w, h, r, color)` | With rounded corners |
| `frame(x, y, w, h, r, color)` | The outline of one, two pixels wide |
| `circle(x, y, d, color)` | A filled circle, `d` across, its box at (x, y) |
| `line(x1, y1, x2, y2, color, width)` | A line |
| `text(x, y, s, color)`, `text_small(...)` | Text, the top left of its line at (x, y) |
| `text_width(s)`, `text_small_width(s)` | Its width |
| `text_height()`, `text_small_height()` | The height of a line |
| `math(x, y, latex, color)` | A formula, set as results are |
| `soft(["New", "", "Level"])` | Turns the soft keys on and labels them, upper row first; `""` is a dead key, `soft([])` turns them off |

A colour is `"background"`, `"surface"`, `"hairline"`, `"text"`, `"muted"`, `"faint"`,
`"result"`, `"error"`, `"cyan"`, `"amber"`, `"tile1"` to `"tile14"`, or `rgb(r, g, b)`. At most
4000 shapes per frame. The fonts have no subscript digits: write `R1`, not `R₁`.

## Input templates

`input(kind, options)` says how the app is typed into; the last call holds. The hui does the
typing and draws its part.

| Kind | Options | The app gets |
|---|---|---|
| `"keys"` | | Every key in `key`. The default |
| `"grid"` | `cols`, `rows`, `digits`, `wrap` | `pick(col, row)` for OK, EXE or a tap. `cursor()` gives `#{col, row}`, `cursor(col, row)` sets it. With `digits`, 1–9 pick the cells of a 3 × 3 grid as the number block lies |
| `"cells"` | `cols`, `rows`, `digits_only` | `entered(#{col, row, value})` when a cell's value is typed and confirmed; `digits_only` takes one digit and moves on |
| `"list"` | `items` | `entered(index)` |
| `"text"` | `label`, `max` (32), `value` | `entered(text)` on EXE or OK |
| `"line"` | `label`, `calc` | `entered(latex)` on EXE: the calculator's input line with SHIFT, ALPHA, UNIT and SYMB. With `calc: true`, what `calc` gives |

Keys a template uses do not reach `key`; the others do.

Two questions open over the app and give their answer as the call's value:

| Function | |
|---|---|
| `confirm(question)` | Yes or No: `true` or `false`; AC is `false` |
| `ask(label, options)` | One line of text, `options` as for `"text"`; the text, or `()` after AC |

## Helpers

| Function | |
|---|---|
| `grid(cols, rows)` | A board laid into the area as large as fits: `#{x, y, w, h, cols, rows, cell}`. `g.cell(col, row)` gives `#{x, y, w, h}`, `g.at(x, y)` the cell under a tap or `()` |
| `side()` | The rest of the area for a score: right of the board where it is wide, below it where not |
| `random(n)` | A whole number from 0 to n − 1 |
| `timer(ms)` | Calls `tick` every `ms`, no faster than 100 ms; `timer(0)` stops it |
| `save(value)`, `load()` | The app's saved data (below); `load()` is `()` when there is none |
| `lang()` | `"de"` or `"en"` |
| `now()` | The time as Unix milliseconds, or `()` before the clock is set |
| `exit()` | Leaves the app |

## The calculator

| Function | |
|---|---|
| `calc(input)` | Works `input` out as the calculator's line takes it, with the open session's settings and variables, and gives `#{ok, value, latex, error}`. The history is left alone. While the engine works the spinner turns; AC drops it |
| `variable(name)` | A variable of the session by its name or symbol, or `()` |
| `insert(latex)` | Leaves the app and puts `latex` at the cursor of the calculation |
| `form(description)` | Opens a form (below) |

## Forms

A tool that asks for values and shows results needs no drawing:

```rhai
// name: Voltage divider
// kind: tool
// api: 1

form(#{
    title: "Voltage divider",
    fields: [
        #{ id: "U",  label: "Input", unit: "V" },
        #{ id: "R1", label: "R1", unit: "Ω" },
        #{ id: "R2", label: "R2", unit: "Ω", value: "10 kΩ" },
    ],
    results: [
        #{ label: "U2", formula: "U*R2/(R1+R2)", unit: "V" },
        #{ label: "I",  formula: "U/(R1+R2)",    unit: "mA" },
    ],
})
```

One field per line, the normal editor in each. EXE on the last field works it out: a number
alone takes the field's unit, a value typed with another unit of the same quantity is taken as
typed (`10 kΩ` into a field in Ω), one of a wrong quantity is an error on its field. The
results appear below, converted to their `unit`; OK on one hands it to the calculation. The
inputs are kept for the next time.

A field can have `value`, `hint` (a line below it) and `choices: ["Steel", "Aluminium"]` (a
list instead of an input).

For more than formulas, give `submit: |v| { ... }` instead of `results`: it gets the inputs by
id, as typed, may use `calc`, and returns a list of `#{label, latex}`, an error text, or
`#{field, error}` for one field. In an app that draws, `form(...)` shows **Done** on the first
soft key, which closes the form and gives the results as a map from label to LaTeX.

## Saved data

`save(value)` keeps one value (a map, an array, numbers, strings, booleans) in
`/apps/data/<id>.json`, at most 64 KB. **OPTN → Delete data** removes it, after a question.
Without a card it lasts until the handheld is switched off. An app that is only a form keeps its
inputs there.

## Limits

An app is a file from someone else; what it can do is limited, whoever wrote it.

| | |
|---|---|
| Reach | The functions on this page only: no files, no network, no settings, no other app's data |
| Time | A budget of operations per callback; over it the app stops with an error |
| Depth | 32 nested calls |
| Size | A file of at most 256 KB; strings of 16 KB; arrays of 10,000 and maps of 1,000 items; 4000 shapes per frame; 64 KB of saved data |

An error opens a page with the message and the line; OK or AC go back.

## Passing an app on

Copy the file from your card to someone else's, or send it any way you like. Quantus keeps no
collection of apps and the handheld fetches none. The `author` line is what the author wrote
and proves nothing; the limits above are what protect you. Saved data stays on your card.

The built-in games are apps like yours, and the best examples:
[tic tac toe](https://github.com/iswearimmrtobi/quantus/blob/dev/firmware/apps/apps/tictactoe.rhai)
and [2048](https://github.com/iswearimmrtobi/quantus/blob/dev/firmware/apps/apps/2048.rhai).
An app with the id of a built-in one is listed beside it, not instead of it.
