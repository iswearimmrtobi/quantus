# Apps and games

State on 2026-11-11: built, and tested in the simulator and on the board; how it was built, step
by step, is in [apps-roadmap.md](apps-roadmap.md). This page describes the design: what an app is, how the hui runs one, the script API, the two first
games; app authors read the site's *Writing apps*.

Handheld only. The desktop app gets none of this.

## Context

Two things are wanted on the handheld:

- **Games**, tic tac toe and 2048 first, on a small engine of their own, so that a further game is
  little work and users can add their own.
- **User-made UIs for special calculations**: a page that asks for the values of one task (a beam,
  a voltage divider, a lab evaluation) in labelled fields and works the result out, so that the
  input is easier than typing the formula each time.

Both are the same thing to the hui: a program that is not part of the firmware, takes the screen
and the keys for a while, and gives them back. This page calls it an **app**.

What exists today:

- Nothing about games, scripts or plugins, anywhere in the repository.
- The screens of the hui are the variants of one enum (`Screen` in
  [`app.rs`](../ui/src/app.rs)); there is no trait a new screen could implement.
- Forms exist, built in: CALC asks for the free variables of an input, and the sheets
  (`sheet::Kind` in [`sheet.rs`](../ui/src/sheet.rs)) are one column of labelled fields that EXE
  on the last field works out.
- MODE has twelve tiles on two pages ([handheld-menus.md](../../design/handheld-menus.md) §7.1);
  *Sitzungen* is a tile that opens a page instead of a mode. SETUP opens with SHIFT + MODE only.
- Related ideas, none of them planned: macros (idea 4), packs of formulas on the card (idea 31)
  and an exam mode teachers can trust (idea 36) in [feature-ideas.md](../../plans/feature-ideas.md).

What shapes the design:

| Fact | Consequence |
|---|---|
| 480 × 320 in Rgb565, drawn in ten bands of which only the changed ones are sent; a band takes about 9 ms, the whole screen about 90 ms ([hardware.md](hardware.md)) | Board games and forms suit the display, fast full-screen animation does not. A timer ticks no faster than about every 100 ms |
| The small unit is planned with 320 × 240 | An app asks for the size of its area and never assumes one |
| Keys arrive as presses only; a repeat is made by the key's source. No release, no held keys, at most two keys at once on the matrix | Games are turn-based or step on a timer. Nothing needs a held key |
| Touch is never the only way in ([unit-keys.md](unit-keys.md)) | Every app works from the keys; a tap is an extra |
| No random numbers anywhere in the firmware | The platform has to give them (`esp_random` on the board) |
| The UI reaches the card only through single methods of `Backend` (wallpapers, sessions) | Listing apps and keeping their data are new methods; an app gets no file access of its own |
| 32 MB PSRAM, about 33 MB of heap free; 7.9 MB for the firmware image; the main task has a 16 KB stack ([plans.md](../../plans/plans.md) §4) | An interpreter fits. Where it runs depends on the stack it needs (§5) |
| The board builds with `panic_abort` | A fault in an app must come back as an error value, never as a panic |

## 1. Decisions

1. **Apps are scripts on the SD card.** A file copied into `/apps/` appears on the handheld. No
   toolchain and no flashing, which is the only way most users could add anything.
2. **The built-in games are scripts too**, the same files, compiled into the firmware so that they
   work without a card. There is one way to write an app, the games test the API, and they are
   the examples users start from.
3. **Two levels for a custom UI**: a **form** that is declared (fields, units, a formula) and
   drawn by the hui in its own look, and **free drawing** on the canvas the games use. Most
   calculation UIs are forms and need no programming; the canvas is there for the rest.
4. **One MODE tile, *Apps***, lists games and tools together, built in first, then the card's.
   It opens a third page of the mode menu, and a tile *Setup* stands beside it, which opens the
   menu SHIFT + MODE opens (§7).
5. **The script language is Lua 5.4**, chosen after the spike of step 0 measured Rhai, the first
   choice, and Lua on the board (ESP32-P4 at 400 MHz, flash in QIO at 80 MHz, WLAN connected):

   | On the board | Rhai 1.27 | Lua 5.4 (mlua) |
   |---|---|---|
   | 2048: compiling the file | 37 ms | 12 ms |
   | 2048: a move with its `draw` | 21–25 ms | 7–10 ms |
   | Tic tac toe: the computer's move on *schwer* | 7–12 ms | 4–6 ms |
   | A loop of 10,000 steps | 63 ms | 8 ms |
   | The time budget (§5) | 200,000 operations, 211 ms | 1,000,000 instructions, 183 ms |
   | The image | +1.04 MB | +0.27 MB |
   | The deepest stack the limits allow | 438 KB | 250 KB |
   | Every limit an error, none a halt | yes | yes |

   Both fit; Lua is about three times as fast and a quarter of the size, and is known from games
   and other calculators. The PC builds Lua from the `lua-src` crate; the board compiles the same
   C sources from `firmware/handheld/lua` with the ESP toolchain, since `lua-src` knows no ESP-IDF
   target. The API in §4 names functions and values, so it carried over from Rhai unchanged but
   for Lua's own words: `formula` for `math`, `math.random` for `random`, `g:cell` and `nil`.

6. **No apps in Prüfungsmodus.** With `Settings::exam` on, the tile says why and opens nothing.
   A tool from the card is a formula sheet, which an exam forbids, and a game has no place there.
   Letting a teacher allow single apps belongs to idea 24.
7. **AC always leaves an app.** The hui handles it before the script sees it, so a script that
   hangs or misbehaves can always be left.
8. **The frame is the hui's.** The top bar is drawn by the hui on every app and no app can change
   or hide it. The soft keys are off until an app turns them on and labels them. The area between
   is the app's (§4.3).
9. **An app picks how it is typed into**, from templates the hui provides: its own handling of
   the keys, the calculator's input line as Physik has it, a line of plain text, a grid with a
   cursor, a list, a question (§4.4). Only the first is the app's own work.
10. **Apps pass from user to user**, as files. There is no store, no catalogue and no download
    from Quantus. An app names its author in its header, and OPTN inside an app shows the credits
    (§2).

## 2. What an app is

One file, `/apps/<id>.lua` on the card; the file's name without the ending is the app's id. In
the simulator the folder is `firmware/sim/sd/apps/`. The first lines are a header in comments,
which the hui reads to list the app without running it:

```lua
-- name: 2048
-- name.de: 2048
-- kind: game
-- api: 1
-- author: Quantus
-- version: 1.0
-- about: Slide the tiles and join equal ones.
-- about.de: Schiebe die Kacheln und verbinde gleiche.
```

- `name`: what the list shows; `name.<lang>` for one language.
- `kind`: `game` or `tool`. The list groups by it.
- `api`: the version of the script API the app was written for. An app that asks for a newer one
  than the firmware has is listed in grey and says so when opened.
- `author`, `version`, `about` (and `about.<lang>`), `license`: the credits, all optional, each
  one line of at most 80 characters. A line may be repeated (`author` twice for two authors).
  The hui shows them and makes nothing of them: a version is text, not compared.
- `tile`: `yes` gives the app a tile of its own in the MODE menu, after *Setup* (§7). Optional;
  without it the app is only in the list.

**Credits.** OPTN inside an app opens a menu of the hui's, over the app, which the app cannot
change:

1. *Info*: the name, the authors, the version, the description, the licence, the API version and
   where the app comes from (*Eingebaut* or *SD-Karte*, with the file's name).
2. *Neu starten*: runs `init` again.
3. *Daten löschen*: removes the app's saved data, after a question.
4. *Beenden*.

OPTN on an app in the list opens *Info* and *Daten löschen* without starting it. An app from the
card with no `author` line says *Autor unbekannt*.

**Passing an app on.** A user copies the file from their card to someone else's, or sends it by
any means and the other copies it into `/apps/`. That is all there is: Quantus keeps no
collection of apps and the handheld fetches none. The saved data in `/apps/data/` is the user's
own and is not part of the app.

A file without a `name` line, over the size limit (§5) or with another ending is left out.

The built-in apps are files in `firmware/apps/apps/`, embedded with `include_str!`. An app on the
card with the id of a built-in one is listed beside it, marked as the card's, and does not replace
it.

**Saved data.** An app can keep one value (a table: the best score, a game in progress, the last
inputs of a form) with `save` and `load`. It is stored as JSON in `/apps/data/<id>.json`, at most
64 KB. Without a card it lasts until the handheld is switched off.

## 3. Architecture

```
MODE → Apps ──► list (Backend::apps) ──► Backend::open_app(id) ──► Box<dyn Applet>
                                                                        │
  App::press ── key ──►  Applet::key  ┐                                 │
  App::tick ── time ──►  Applet::tick ├─► script callbacks ─► Host: calc, random, save, clock
  Frame::new ◄─ shapes ─ Applet::draw ┘
```

**In the hui (`firmware/ui`)**, which stays free of the interpreter:

- `trait Applet` in a new `applet.rs`: `key(&mut self, key: AppKey, host: &mut dyn Host)`,
  `tick(&mut self, now_ms: u64, host: &mut dyn Host) -> bool` (whether to redraw),
  `draw(&self, canvas: &mut Canvas)`, `tap(&mut self, point, host)`, `soft_keys(&self)` (none
  hides the strips), `input(&self) -> Input` (the template in use, §4.4), `info(&self)` (the
  credits) and `leave(&mut self, host) -> Option<String>` for a value handed to the calculation.
- `Screen::App`, with the applet itself in `App::running` (`Running`: the applet, its
  `AppInfo`, the screen it was opened from), since `Screen` is cloned and compared and a
  `Box<dyn Applet>` is neither. `App::press`, `App::tick`, `Frame::new` and `render::target`
  each get one branch for it; that is all the hui knows about apps.
- The hui draws the top bar (the app's name where a mode's name stands, the clock and the
  battery as everywhere), the soft-key strips when the app has any, the template's part (the
  input line, the grid's cursor, a list, a question) and the OPTN menu. The applet draws the rest.
- `Canvas`: the app's area (below the top bar, above the soft-key strips if they show) as a list
  of the
  `Shape`s the hui already draws with ([`render/mod.rs`](../ui/src/render/mod.rs)): `Fill`,
  `Round`, `Circle`, `Path`, `Text`, `Math`. Since an app only adds shapes to the frame, the band
  renderer sends just the bands that changed, as for every other screen.
- `trait Host`: what an app may ask of the device. The hui implements it over its `Backend` and
  its own state: `calc`, `variable`, `random`, `now`, `load`, `save`, `lang`.
- `AppKey`: the keys an app gets, without the modifier layers: arrows, OK, EXE, DEL, digits, the
  operators, the soft keys. AC, MODE and OPTN are not among them.
- `Mode::Apps`, the thirteenth tile and the first of page 3, and `Mode::Setup`, the fourteenth,
  each with a colour in `colors::TILES`. `choose` opens the list as it opens *Sitzungen*, and
  SETUP's menu for the other, as SHIFT + MODE does; neither becomes the mode. The list is a menu
  level filled at runtime, as the session and network lists are (`Edit` in
  [`menu.rs`](../ui/src/menu.rs)).
- The form (§4.7) is an `Applet` written in Rust inside the hui, since it reuses the sheets'
  fields, editor and drawing ([`render/sheet.rs`](../ui/src/render/sheet.rs)).

**New on `Backend`** ([`backend.rs`](../ui/src/backend.rs)), each with a default that does
nothing, so `Stub` and the tests need no change:

```rust
fn apps(&mut self, lang: Lang) -> Vec<AppInfo>;                  // id, name, kind, built in?, usable?
fn open_app(&mut self, id: &str) -> Result<Box<dyn Applet>, String>;
fn app_data(&mut self, id: &str) -> Option<String>;              // the saved JSON
fn store_app_data(&mut self, id: &str, json: &str);
fn random(&mut self) -> u32;
```

**New crate `firmware/apps` (`quantus-apps`, in the main workspace)**: the interpreter, the
script API, the limits, the embedded games. It depends on `quantus-ui` for `Applet`, `Canvas` and
`Host`, and gives `list(card_dir)` and `open(id, card_dir)`.

**The script runs on a thread of its own.** `confirm`, `ask`, `form` and `calc` give their
answer as the call's value, but the hui's loop (`App::press`, `App::tick`) cannot take keys
while a callback has not returned. So the script's engine and its compiled file live on their
own thread (`Script` in `firmware/apps/src/script.rs`), and the main side sends it a callback
and waits for it to *yield*: the callback and `draw` are done (`Yield::Done`, with what `draw`
drew and what the script set), or the script stands in a host call that needs the hui
(`Yield::Asks`, a question or a calculation) until the answer resumes it, or it failed. What
crosses the channel is plain data: Lua's values are not `Send`. The thread gives the
interpreter the stack it needs, and a callback that does not come back in time counts as hung:
the main side stops waiting and the operation budget ends the thread.

**`firmware/engine`** implements the new `Backend` methods with `quantus-apps`, for the simulator
and the board alike, as it does for sessions. Random numbers come from `esp_random` on the board
and from a generator in the simulator that `--seed` fixes for tests. `calc` goes to `api::calc`,
which works an input out without writing it into the history.

**Nothing changes in `quantus-core`.** Games are the handheld's alone. The description of a form
(§4.7) is kept free of anything the hui alone knows, so that the core could read the same files
later, for the desktop as well (ideas 5 and 19).

## 4. The script API, version 1

### 4.1 Callbacks

An app defines the functions it needs, as global functions; all are optional. One without
`draw` is only a form (§4.7). Positions in a grid, cells and lists count from 1, as Lua's lists
do; pixels count from 0.

| Function | Called |
|---|---|
| `init()` | Once, when the app opens. Sets up the state, loads saved data |
| `key(k)` | For every key press. `k` is the key's name |
| `tick(ms)` | On the timer the app set with `timer`. `ms` is the time since the app opened |
| `draw()` | After any callback, to describe the whole screen |
| `tap(x, y)` | For a tap inside the app's area |
| `entered(value)` | When an input line or a line of text is confirmed (§4.4) |
| `pick(col, row)` | When OK is pressed on a cell of the grid (§4.4) |
| `leave()` | When the app closes (AC, or MODE). The place to `save` |

The app's state is its variables: one Lua state lives as long as the app is open, so `local`
variables at the top of the file keep their values from one callback to the next.

```lua
-- name: Counter
-- kind: tool
-- api: 1

local n = 0

function key(k)
  if k == "up" then n = n + 1 end
  if k == "down" then n = n - 1 end
end

function draw()
  clear("background")
  text(20, 40, "Count: " .. n, "text")
end
```

### 4.2 Keys

`key` gets a name: `"up"`, `"down"`, `"left"`, `"right"`, `"ok"`, `"exe"`, `"del"`, `"0"` to
`"9"`, `"+"`, `"-"`, `"*"`, `"/"`, `"="`, `"."`, `"soft1"` to `"soft12"`. SHIFT, ALPHA and UNIT
reach no script; they work inside an input line or a line of text (§4.4) as they do in a
calculation, and nowhere else in an app. AC leaves, MODE leaves and opens the main menu, OPTN
opens the app's menu with the credits (§2). A held key repeats if its source repeats it (the USB
keyboard and the simulator do).

Which keys `key` gets depends on the input template (§4.4): all of them with `"keys"`, the ones
the template does not use with the others.

### 4.3 The screen and drawing

The screen of an app has three parts, from the top:

| Part | Whose | |
|---|---|---|
| Top bar, 26 px | The hui's, fixed | The app's name, the clock, the battery, the spinner. No app changes, covers or hides it |
| The app's area | The app's | Everything `draw` describes; the hui adds the template's part (§4.4) |
| Soft keys, 44 px | The app turns them on and labels them | Two rows of six keys on the big unit, of five on the small one. Off, the area reaches the bottom of the screen |

`soft(["Neu", "Zurück"])` turns the soft keys on and labels them from the left, the upper row
first; an empty text leaves a key blank and dead. `soft([])` turns them off again. The labels may
change at any time (*2 Spieler* / *Computer*). A press comes as `key("soft1")` and so on. An app
that names more keys than the unit has gets them in pages, turned as the calculation's soft keys
are.

The area is 480 × 294 on the big unit without soft keys and 480 × 250 with them; on the small
unit it is 320 wide and lower. **An app lays itself out for the area it gets**: it asks
`width()` and `height()` in `draw` and does not assume a size. For the usual case of a board with
something beside it, `grid` and `side` (§4.5) do the arithmetic: the side panel is to the right
of the board where the area is wide, below it where it is not. The built-in games use them and
so have a layout for both units.

Coordinates are pixels from the top left of the app's area. `draw` describes the whole area each
time; the renderer finds what changed.

| Function | |
|---|---|
| `width()`, `height()` | The size of the area as it is now |
| `clear(color)` | Fills the area |
| `rect(x, y, w, h, color)` | A filled rectangle |
| `round(x, y, w, h, r, color)` | With rounded corners |
| `frame(x, y, w, h, r, color)` | The outline of one, two pixels wide, inside it |
| `circle(x, y, d, color)` | A filled circle, `d` across, its box's top left at (x, y) |
| `line(x1, y1, x2, y2, color, width)` | A line; `width` 1 when left out |
| `text(x, y, s, color)`, `text_small(...)` | Text in the hui's two fonts, the top left of its line at (x, y); a number as `tostring` writes it |
| `text_width(s)`, `text_small_width(s)` | Its width, for centring |
| `text_height()`, `text_small_height()` | The height of a line, for centring |
| `formula(x, y, latex, color)` | A formula, set as the calculator sets results, its top left at (x, y) (`math` is Lua's library) |

A colour is a name from the hui's palette (`"background"`, `"surface"`, `"text"`, `"muted"`,
`"result"`, `"error"`, `"cyan"`, `"amber"`, `"tile1"` to `"tile14"`), so an app follows the
theme, or `rgb(r, g, b)`.

### 4.4 Input templates

An app says how it is typed into with `input(kind, options)`, in `init` or at any later time; the
last call holds. The hui does the typing, the cursor and its drawing, and the app gets the
outcome. `"keys"` is the default.

| Kind | What the user does | What the hui draws | What the app gets |
|---|---|---|---|
| `"keys"` | Presses keys | Nothing | `key(k)` for every key. The app's own input, for a game that steers with the arrows |
| `"line"` | Types as in Physik: numbers with units and prefixes, fractions, powers, constants, with SHIFT, ALPHA, UNIT and SYMB (SYMB's symbols take the soft keys while open; no unit bar) | The input line at the bottom of the area, with a `label` in front | `entered(value)` on EXE, `value` as `calc` takes it. `options.calc: true` works it out first and gives what `calc` gives |
| `"text"` | Types plain text: letters, digits, the space, as when a session is renamed | One line with a `label`, at the bottom of the area | `entered(text)` on EXE or OK. `options.max` limits the length (32 if not named), `options.value` stands in it at first |
| `"grid"` | Moves a cursor over `options.cols` × `options.rows` cells with the arrows or a tap, picks with OK or EXE | The cursor's frame on the cell, over the app's drawing | `pick(col, row)`. `cursor()` gives the cell it is on as `{col, row}`, `cursor(col, row)` sets it. `digits = true` lets 1–9 pick a cell of a 3 × 3 grid directly, laid out as the number block is. `wrap = true` lets the cursor go round the edges |
| `"cells"` | The same grid, and types a value into the cell under the cursor, as in the Matrizen grid | The cursor and the editor in the cell | `entered({col, row, value})` when a cell is confirmed. `digits_only = true` takes one digit per cell and moves on to the next (a row of digits typed in one go; Sudoku uses `"grid"`, §6.3) |
| `"list"` | Goes through `options.items` with ↑/↓ or picks with a digit | The list, as a menu of the hui's, in the area | `entered(index)` |

Two questions need no template to be set and no callback; they open over the app and give their
answer as the call's value:

| Function | |
|---|---|
| `confirm(question)` | A question with *Ja* and *Nein*, as the hui asks before deleting. `true` or `false`; AC is `false` |
| `ask(label, options)` | One line of plain text over the app (a name for a high score), with `options` as for `"text"`. The text, or `nil` when it was left with AC |

With every template but `"keys"`, the keys the template uses do not reach `key`; the others do
(the soft keys always, the digits in a grid without `digits`). An input line or a line of text
takes part of the area, so `height()` is smaller while it shows. `form` (§4.7) is the template
for a whole page of fields and stands for itself.

### 4.5 The games' helpers

| Function | |
|---|---|
| `grid(cols, rows)` | Lays a board into the area, as large as fits: centred where the area is narrow, at the left where a side panel fits beside it. Gives the board as `{x, y, w, h, cols, rows, size}` (`size` the side of a cell), with `g:cell(col, row)` as `{x, y, w, h}` and `g:at(x, y)`, the cell under a tap as `{col, row}` or `nil`. The `"grid"` and `"cells"` templates use the same cells |
| `side()` | The part of the area the board leaves, as `{x, y, w, h}`: right of the board on a wide area, below it on a narrow one. For the score, whose turn it is, a hint |
| `math.random(n)` | Lua's own, from 1 to n (`math.random(m, n)`, `math.random()` as in Lua), but from the host's generator, which `--seed` fixes in the simulator; `math.randomseed` is not there |
| `timer(ms)` | Calls `tick` every `ms`; no faster than 100 ms. `timer(0)` stops it |
| `save(value)`, `load()` | The app's saved data (§2); `load()` gives `nil` when there is none (Lua's own `load`, which runs code from text, is not there) |
| `lang()` | `"de"` or `"en"`, the hui's language |
| `now()` | The time as Unix milliseconds, or `nil` before the clock is set |
| `exit()` | Leaves the app |

### 4.6 The calculator from a script

| Function | |
|---|---|
| `calc(input)` | Works `input` out with the open session's settings and variables and gives `{ok, value, latex, error}`. `input` is what the calculator's line takes; the history is left alone |
| `variable(name)` | A variable of the session by its name or its symbol, as `calc` gives values, or `nil` |
| `insert(latex)` | Leaves the app and puts `latex` at the cursor of the calculation |
| `form(description)` | Opens a form (below) and gives its result when it closes |

`calc` waits for the engine. A calculation that takes long holds the app, with the spinner in the
top bar; AC drops it and leaves.

### 4.7 Forms

A tool that only asks for values and shows results is a description, with no callbacks:

```lua
-- name: Spannungsteiler
-- kind: tool
-- api: 1

form({
  title = "Spannungsteiler",
  fields = {
    {id = "U",  label = "Eingangsspannung", unit = "V"},
    {id = "R1", label = "R₁", unit = "Ω"},
    {id = "R2", label = "R₂", unit = "Ω", value = "10 kΩ"},
  },
  results = {
    {label = "U₂", formula = "U*R2/(R1+R2)", unit = "V"},
    {label = "I",  formula = "U/(R1+R2)",    unit = "mA"},
  },
})
```

The hui draws it as it draws a sheet: one field per line with its label and unit, the normal
editor in each field (fractions, powers, units and prefixes typed as in a calculation), ↑/↓
between fields, EXE on the last field works it out. The results appear as an answer below, each
set as a formula, converted to its `unit` if one is named. OK on a result hands it to the
calculation, as `insert` does. The inputs are saved with the app's data and stand in the fields
the next time.

A field can carry `value` (what stands in it at first), `hint` (a line below it) and
`choices = {"Stahl", "Alu"}` (a list instead of an input). A unit on a field is the one assumed
for a bare number; a value typed with another unit of the same quantity is taken as typed, and
one of a wrong quantity is an error on the field, which the engine's dimension check finds.

For more than formulas, `results` is replaced by `submit = function(v) … end`, which gets the
values as a table (by the fields' ids, as typed), uses `calc` as it likes, and gives the list of
results (`{label, latex}`), an error text for the form, or `{field, error}` for a field.

A script that is only a `form(...)` call has no `draw`; its inputs are its saved data. In a
script that draws, `form(...)` puts *Fertig* on the first soft key, which closes the form and
gives the results as a table from label to LaTeX.

## 5. Limits and safety

An app is a file from a card, possibly from someone else. It may be wrong; it must not be able to
harm the handheld or the user's data.

- **What a script can reach**: the functions of §4 and Lua's `string`, `table`, `math` and
  `utf8`, nothing else. No `io`, `os`, `debug`, `package` or `require`; no `load`, `dofile` or
  `loadfile` to run code from text or files (`load` is the app's saved data); `print` does
  nothing. No files, no network, no settings, no other app's data, no sessions other than the
  open one through `calc` and `variable`.
- **Time**: each callback may run 1,000,000 Lua instructions, about 200 ms on the board; a hook
  counts them every 1000. Over it, the callback stops with an error on its line. `pcall`,
  `xpcall` and coroutines are left out, since they could catch that error and run on for ever;
  any error ends the callback. AC works at any time (decision 7), and the main side stops
  waiting after 3 s.
- **Memory**: 4 MB for the script's Lua state, which ends an endless recursion or a string grown
  without end with an error; a script file of at most 256 KB; saved data of at most 64 KB;
  tables nested at most 32 deep in what leaves the script; 4000 shapes per frame.
- **The frame**: the top bar, AC, MODE and OPTN are the hui's. An app cannot draw over the top
  bar, pose as another screen of the calculator there, or hide its own credits.
- **A stranger's file**: apps come from other users, with nobody in between who checks them. The
  sandbox is what protects the user, so it does not depend on the author being honest; the
  `author` line is what the author wrote and proves nothing.
- **Errors**: a script error opens a page with the message and the line, and OK or AC goes back
  to the list. The host returns errors and never panics, since a panic halts the board.
- **Stack**: Lua calls Lua without the C stack, but a call that goes through C (a metamethod,
  `string.gsub` with a function, a form's `submit`) nests on it, up to Lua's limit of 200; that
  took about 250 KB on the board. Running over the stack halts the board instead of giving an
  error, so the script's thread has 1 MB.
- **Memory on the board**: internal RAM is scarce while WLAN is on, so the script's thread takes
  its stack and everything it allocates from PSRAM (`firmware/handheld/src/main.rs`: the spawn
  hook and the allocator). With small allocations in internal RAM, compiling a game took the
  internal heap down to a few bytes and every allocation of the script fell back to PSRAM
  slowly; the measurement of step 0 found it.
- **Prüfungsmodus**: no app opens (decision 6); CATALOG's search leaves apps out.

## 6. The first games

All five are built in, all are scripts, and all stay within what the display does well: a move
changes a few cells, so a few bands are sent. Each keeps its rules in global functions
(`slide`, `best`, `conflicts`, `open`, `advance`), which the tests in `firmware/apps/tests/`
call without the host.

### 6.1 Tic tac toe

- The 3 × 3 board laid out with `grid(3, 3)`; whose turn it is and the score of the round in
  `side()`, beside the board on the big unit and below it on the small one.
- Input is the `"grid"` template with `digits`: the arrows move the cursor, OK or EXE sets the
  mark, the digits 1–9 set it directly, laid out as the number block is (7 at the top left).
- Soft keys: *Neu*, *2 Spieler* / *Rechner* (*Computer* does not fit a soft key), and the level,
  *leicht* or *schwer* (leicht: random; schwer: never loses, by searching the game tree, which
  has at most 9! sequences of moves and in practice far fewer). Each shows what is set.
- A line of three is marked, the result named, and any key starts the next round with the other
  side beginning.
- Saved: the mode, the level, the score.

### 6.2 2048

- The 4 × 4 board with `grid(4, 4)`; the score and the best score in `side()`.
- Input is `"keys"`: the arrows slide the tiles. A move that changes nothing is ignored. After a
  move a 2 (nine
  times in ten) or a 4 appears on a free cell.
- No sliding animation in version 1: the board is redrawn in its new state, and the tile that
  appeared is outlined. On the dev kit a move takes 7–10 ms in the script and 55–75 ms to send
  the six to eight bands that changed, so a step of an animation costs about 80 ms; whether two
  or three steps on the timer look right waits for the product's panel.
- Tiles are coloured by value from the tile palette; numbers of four and five digits use the
  small font.
- Soft keys: *Neu* (asks first with `confirm` when a game is running), *Zurück* (one move back,
  once).
- Reaching 2048 says so and the game goes on. No free cell and no possible merge ends it.
- Saved: the board, the score, the best score, so that a game survives switching off.

### 6.3 Sudoku

- The 9 × 9 board with `grid(9, 9)`, thick lines between the boxes; the level, the time, the
  best time and *Gelöst!* in `side()`.
- Input is the `"grid"` template without `digits`, so the digits reach `key`: 1–9 write into
  the cell under `cursor()`, 0 and DEL clear it, OK or a tap counts it up (for touch). Givens
  cannot be changed. `"cells"` with `digits_only` would move the cursor on after every digit.
- The puzzles are six per level (*leicht* about 38 givens, *mittel* 30, *schwer* 25), made
  with a single solution outside the firmware. A new game shuffles one: the digits, the rows
  within a band, the bands, the same for the columns, and a transposition, which keeps it a
  Sudoku with one solution. Generating one with a check for uniqueness would not fit a
  callback's budget.
- A digit repeated in its row, column or box is drawn in the error colour; a full board
  without one is solved, whatever the solution was.
- Soft keys: *Neu*, the level (starts a new game on the next one), *Zurück* (any number of
  steps). Both ask first while a game is going on.
- The time ticks every second on the timer. Saved: the puzzle, the entries, the time, the
  level, the best time per level.

### 6.4 Minesweeper

- *leicht* 9 × 9 with 10 mines, *mittel* 16 × 12 with 30, *schwer* 20 × 12 with 45, each laid
  out with `grid`; the mines left, the time and the best time in `side()`.
- Input is the `"grid"` template: OK, EXE or a tap opens a cell, 0 or `.` sets or removes a
  flag on the cursor's cell, and the third soft key (*Öffnen* / *Flagge*) makes a pick set
  flags, for touch. OK on an open number with as many flags around it as it counts opens the
  rest around it.
- The mines are laid on the first opening, never on that cell or around it, so it opens an
  area. Cells without a mine near open their neighbours, by a list of cells to do instead of
  recursion.
- A mine ends the game: the mines are shown, the wrong flags crossed out. Any pick then starts
  a new one.
- Soft keys: *Neu*, the level, the mode. Saved: the level, the best times, a game in progress.

### 6.5 Snake

- A 16 × 12 board with `grid`, so that the side panel fits beside it on the big unit; the
  score and the best score in `side()`.
- Input is `"keys"`: the arrows (or 8, 2, 4, 6) steer. The first arrow starts it. Turns wait in
  a queue of two for the next steps, and a turn back into the snake is ignored.
- It steps on the timer, every 150 ms at first and 2 ms faster with each piece of food, down to
  the timer's 100 ms. A step changes the head, the tail and the food, a few bands.
- The wall and the snake's own body end it; the cell the tail leaves may be entered. Any key
  then sets up the next game.
- Soft keys: *Neu*, *Pause* / *Weiter*. Saved: the best score only; a game ends when the app
  closes.

## 7. Where apps show up

- **MODE, page 3**: tile 13 *Apps* and tile 14 *Setup*. The page is added for them; ↑/↓ and a
  swipe reach it as they reach page 2, and the cursor picks its tiles, as it picks 10 to 12
  (the digits reach 1–9). *Setup* opens the menu of SHIFT + MODE and leaves the mode as it is;
  it is there for whoever looks for the settings among the tiles, and works in Prüfungsmodus.
  Ungleichungen and Geometrie take the next places on the page when they come.
- **MODE, after Setup**: a tile for each app whose header says `tile: yes`, in the list's order
  by name, numbered on from 15; more of them add pages. The tile shows the app's name, OK or a
  tap starts it, AC comes back to the tile. The MODE menu reads them each time it opens, so a
  file copied to the card shows up then. None in Prüfungsmodus.
- **MODE → Apps**: the list, in two groups *Spiele* and *Werkzeuge*, built in first, each line
  with the app's name and its author. A digit picks, OK opens, OPTN shows the credits, ← or AC
  goes back. An empty card adds the line *Eigene Apps: Ordner /apps auf der SD-Karte*.
- **CATALOG's search** finds apps by name ([plans.md](../../plans/plans.md) §6); a hit opens
  the app.
- **SETUP → Gerät → Speicherplatz** counts `/apps/` with the rest of the card.
- An app that hands a value back (`insert`, or OK on a form's result) returns to the calculation
  of the mode that was open, with the value at the cursor.

## 8. Later

- The unit bar on an app's input line (the `"line"` template): units come from the UNIT layer for
  now, and the soft keys stay the app's (apps-roadmap.md F9).
- More games: Memory.
- Pencil marks in Sudoku, and marking the row, column and box of the cursor, which needs the
  script to hear of a cursor moved by the template.
- Key release and held keys, once the keypad's driver exists and can report them.
- Pictures: small bitmaps from the app's file or the card.
- Animation in steps, when the redraw time of the product's panel is known.
- Sending an app from one handheld to another, or to the handheld over WLAN or the link to the
  desktop, instead of moving the card. Still from user to user.
- Single apps allowed in Prüfungsmodus by a teacher's profile (idea 24).
- Forms read by `quantus-core`, for the desktop as well, together with the packs of idea 19.
- An app's own texts in several languages without `if lang() == …`.

## 9. Open questions

- **Long calculations.** `calc` holds the app. Whether a script needs a way to start one and be
  called when it is done depends on what tools users write.

Settled on 2026-11-11:

- **The layout.** The top bar is the hui's and fixed, the soft keys are the app's to turn on and
  label, and an app lays itself out for the area it gets, on both units (decision 8, §4.3).
- **Text input.** An app picks an input template: its own keys, the calculator's line, plain
  text, a grid, cells, a list, and the questions `confirm` and `ask` (decision 9, §4.4).
- **Sharing.** From user to user, as files; the header carries the credits and OPTN shows them
  (decision 10, §2).
- **Tiles beyond nine.** A third page, with *Apps* and *Setup* (§7).
