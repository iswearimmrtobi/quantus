# Apps and games: the implementation roadmap

State on 2026-11-11: nothing is built. [apps.md](apps.md) is the design; its §8 names twelve steps.
This page breaks each of them into commits: what changes, in which files, and how it is checked.
It was written against the code as it stands, and §1 lists where the code asks for something
other than what apps.md says.

One box is one commit on `dev`. A step of apps.md §8 is ticked there once all its boxes here are.
Checks are the step's own tests (`cargo test -p <crate> --test <name>`) and a look in the
simulator, never the whole workspace.

## 1. What the code says about the design

Found while reading the hui, the engine, the simulator and the board's firmware. Each one is
settled in the step named, and apps.md is corrected in the same commit.

| # | Finding | Consequence | Settled in |
|---|---|---|---|
| F1 | `confirm`, `ask`, `form` and `calc` give their answer **as the call's value** (apps.md §4.4, §4.6). The hui's loop (`App::press`, `App::tick`) cannot take keys while a script callback has not returned | A script cannot run on the hui's thread. It runs on a thread of its own and the main thread waits for it to *yield*: the callback is done, or it stands in a host call that needs the hui (a question, a calculation). The same thread gives the stack an interpreter needs (the main task has 16 KB, `CONFIG_ESP_MAIN_TASK_STACK_SIZE`) and lets AC leave a script that hangs. The alternative, answers through callbacks (`confirmed(yes)`), makes every game harder to write | 0.3, 4.2 |
| F2 | `Screen` derives `Clone`, `PartialEq` and `Eq` ([`app.rs`](../ui/src/app.rs)), and `ui/tests/sheets.rs` clones it. `Screen::App(Box<dyn Applet>)` can be none of these | `Screen::App` carries nothing; the running app is a field of `App` (`running: Option<Running>`) | 1.2 |
| F3 | On the board the engine lives on its worker thread (`handheld/src/engine.rs`) and `Board` in `main.rs` is the `Backend`; in the simulator `Sim` is. Wallpapers and storage are done by `Board` and `Sim` themselves, not by `firmware/engine` | `Board` and `Sim` implement `apps`, `open_app`, `app_data`, `store_app_data` and `random` with `quantus-apps`, so apps also work in the board's build without the engine. `firmware/engine` only adds the calculation for `calc` (step 8) | 4.6 |
| F4 | `api::calc` is CALC: it **assigns** its values to the session's variables before it evaluates. `Engine::calculate(.., record: false)` already works an input out without the history | `calc` of the script API uses `calculate` with `record: false`. A form's fields must not become session variables: the form substitutes them into the formula | 8.1, 9.3 |
| F5 | `WIDTH` and `HEIGHT` are constants, 480 × 320 ([`render/mod.rs`](../ui/src/render/mod.rs)). There is no small screen yet, only `App::soft_columns` 5 or 6 | `Canvas` is made for an area it is given. "Both sizes" in the tests means a canvas of 320 × 214 and five soft keys, not a small screen; the real one waits for the small unit | 1.1 |
| F6 | `colors::TILES` has twelve entries, and `PAGES`, `tile_rect`, `dot_rect`, `modes_key` and `render::target` all count with `Mode::ALL.len()` | Fourteen tiles make three pages without further work, once `TILES` has fourteen colours. The palette names of the API are `"tile1"` to `"tile14"`, not 13 | 2.1 |
| F7 | The soft-key labels are laid out as LaTeX (`render/soft.rs`: `latex::parse(&label)`) | An app's label is plain text: it is drawn as text, or escaped, so `2 Spieler` keeps its space and `_` and `^` mean nothing | 1.4 |
| F8 | `Frame::new` pushes the soft strips and the top bar **after** the screen's shapes | An applet's shapes drawn first cannot cover the bar or the strips. No clipping of shapes is needed for decision 8, only this order and a limit on their number | 1.3 |
| F9 | The editor, the unit bar and SYMB belong to `App` (`editor`, `units`, `symbols`, `calc_key`) and are tied to `Screen::Calc`. The unit bar and SYMB use the soft keys | The `"line"` template needs an editor of its own and a rule for the soft keys while it has the cursor: the unit bar shows if the app has set no soft keys, otherwise the app's stay and units come from UNIT and CATALOG | 3.4 |
| F10 | `App::search` is built once per language and mode | Apps in the index need it rebuilt when the list of apps or `Settings::exam` changes | 10.1 |
| F11 | The simulator has no `--seed`, and with `--png` it keeps the sessions in memory | `--seed <n>` is new; with `--png` the apps' data stay in memory too | 4.6, 6.2 |
| F12 | `sim` and `handheld` are workspaces of their own with their own `Cargo.lock` | A new dependency changes three lock files; whether `npm run licenses` sees the firmware's crates is checked in 11.4 | 0.1, 11.4 |

## 2. Order and milestones

apps.md §8 puts all input templates in step 3. The games need three of them (`"keys"`, `"grid"`,
`confirm`); the others are for tools. Step 3 is split so that the games are not held up:

```
0 ─► 1 ─► 2 ─► 3a ─► 4 ─► 5 ─► 6 ─► 7 ─► 3b ─► 8 ─► 9 ─► 10 ─► 11
M0   M1        │          M2   M3   M4          └─ M5 ─┘         M6
```

| Milestone | After | What can be shown |
|---|---|---|
| M0 | 0 | The numbers of the interpreter on the board; Rhai or Lua decided |
| M1 | 2 | MODE page 3, the Apps list, a demo applet written in Rust inside the frame |
| M2 | 5 | Tic tac toe, in the simulator and on the board |
| M3 | 6 | 2048 too, both games keep their state across a restart. **The games are done** |
| M4 | 7 | A file copied to `/apps/` shows up and runs |
| M5 | 9 | Tools: the calculator from a script, forms |
| M6 | 11 | Search, touch, the pages for app authors |

Sizes below: **S** is an evening, **M** a day or two, **L** several days. They compare the steps
with each other and promise nothing.

## 3. The steps

### Step 0. Spike: the interpreter on the board (M)

Throwaway code on a branch of its own, in the `handheld` build. Only the numbers and the decision
are kept.

- [x] **0.1 Rhai builds for the board.** `rhai` as a dependency of `firmware/handheld` behind a
  feature `spike-apps`, a serial command that runs a script given as text. Features to try, to be
  checked against Rhai's documentation: no `unchecked` (the limits must stay), no `sync`,
  `only_i32` or `f32_float` for the P4, `no_module`, `no_custom_syntax`, `no_time`. Record: the
  growth of the image against the 7.9 MB `factory` partition (`partitions.csv`), the build time.
- [ ] **0.2 A script that plays 2048.** Board as an array in a map, `slide` in four directions,
  the state as `this` in `init`, `key` and `draw` (`call_fn` with a bound `this`; confirm the
  mechanism). `draw` only counts the shapes it would make. Record per callback: time, stack high
  water mark (`uxTaskGetStackHighWaterMark`), heap and internal RAM, as `report_memory` in
  `handheld/src/engine.rs` prints them.
- [ ] **0.3 On a thread, yielding.** The script on a thread started as the engine's worker is
  (`std::thread::Builder::stack_size`), the main loop sending a key and waiting for the answer on
  a channel; one host function (`confirm`) that blocks the script until the main loop answers.
  Record: the stack size that is enough (start with 32 KB), the round trip in ms, whether the
  thread's stack comes from internal RAM (380–440 KB free, 116 KB at the lowest during a sketch)
  or can come from PSRAM, and that the thread ends cleanly when its channel closes.
- [ ] **0.4 The limits hold.** `while true {}` ends by the operation budget
  (`set_max_operations`, `on_progress`), a recursion without end by `set_max_call_levels`, a
  string doubled without end by `set_max_string_size`; each comes back as an error value with a
  line number. The board builds with `panic_abort`: nothing may panic. Record the budget that
  gives about 200 ms.
- [ ] **0.5 Decide and write down.** The table of numbers into apps.md decision 5; §3 and §5 of
  apps.md rewritten for the script thread (F1). If Rhai fails a criterion, the same four boxes
  with `mlua` before going on.

Numbers so far (rhai 1.27, release, `opt-level = "s"`, image from `espflash save-image`; the
same binary without Rhai is 580,544 B): with `only_i32`, `f32_float`, `no_module`,
`no_custom_syntax`, `no_time`: 1,624,720 B, **+1.04 MB**; with `no_closure` and `no_optimize` too:
1,486,368 B, **+906 KB**. Both are over the 600 KB criterion but a small part of the 7.9 MB
`factory` partition. A clean build of the dependencies took 1 m 26 s, the crate alone 20–50 s.
0.2 to 0.4 wait for a board: `/dev/ttyACM0` on the development PC is an ST-Link, not the dev kit.

Criteria for Rhai: a 2048 move with `draw` under 30 ms; under 600 KB of image; a stack of at most
64 KB; every limit comes back as an error.

### Step 1. hui: the frame for apps (L)

Everything here is in `firmware/ui`, with no interpreter. The applet under test is written in
Rust in `ui/tests/apps.rs`.

- [x] **1.1 `applet.rs`: the types.** `trait Applet` (`key`, `tick`, `draw`, `tap`, `soft_keys`,
  `input`, `info`, `leave`), `trait Host`, `AppKey` with `AppKey::from(Key)` (arrows, OK, EXE,
  DEL, digits, operators, `=`, the comma, `Soft(n)`; `None` for the rest), `AppInfo` (id, name,
  kind, authors, version, about, licence, api, built in, usable), `Canvas`: built for a
  `Rectangle`, with `width()`, `height()` and one method per shape of apps.md §4.3 that pushes a
  `Shape` moved by the area's origin, and a limit on their number (F5, F8). `Palette`: a colour's
  name to `Rgb565`, `rgb(r, g, b)`. Unit tests: every `Key` maps as §4.2 says; a canvas of
  480 × 250 and one of 320 × 214 place the same call at the right screen points.
  Built without `info` on the trait: the `AppInfo` the list gave travels in `Running`. Added:
  `frame` (an outline, `Shape::Outline`, for the grid's cursor and 2048's new tile) and
  `text_height`; text and formulas are placed by their top left, like every other shape.
- [x] **1.2 `Screen::App` and `App::running`.** `Running { applet, info, menu: Option<..>,
  back: Box<Screen> }` as a field of `App` (F2). `App::open_applet(applet, info)` for the tests
  and step 2. `App::press`: on `Screen::App`, AC leaves to `back`, MODE leaves to
  `Screen::Modes`, everything else goes through `AppKey::from` to `Applet::key`; modifiers are
  not waiting there. `leave` is called on every way out. Test: keys arrive in order; AC returns
  to the screen the app was opened from; SHIFT and ALPHA reach nothing.
- [x] **1.3 Drawing.** `Frame::new`: a branch that makes the `Canvas` for the area (below `BAR`,
  above the strips if `soft_keys()` has any), calls `Applet::draw` and appends the shapes;
  `backdrop` gives none for an app. `bar.rs`: the app's name as the title, `goes_back` true.
  `App::tick`: `Applet::tick(now_ms)`, its redraw flag into `changed`. Test: the shapes land in
  the frame below the bar; a shape drawn at y = −40 is covered by the bar's fill; a ticking
  applet makes `tick` return true.
- [x] **1.4 Soft keys.** `Slot::App(String)` in [`soft.rs`](../ui/src/soft.rs); `soft::bar`
  gives the applet's labels on `Screen::App`, in pages with `Slot::Page` when there are more
  than `2 × soft_columns`; `App::soft_key` sends `AppKey::Soft(n)` counted over all pages; an
  empty label is dead. The label as plain text (F7). Test: with labels the area is 250 high,
  without 294; thirteen labels page; `soft_columns = 5` gives rows of five.
- [x] **1.5 The app's menu.** OPTN on `Screen::App` opens a list of the hui's over the app:
  *Info* (a page built from `AppInfo`, *Autor unbekannt* without an author), *Neu starten*
  (`Applet::restart`, added to the trait), *Daten löschen* (a question first; the data come in
  step 6, the entry is there and says *Keine Daten*), *Beenden*. While it is open the applet
  gets no keys. Texts: `hui.apps.info`, `.restart`, `.delete_data`, `.quit`, `.author_unknown`,
  `.built_in`, `.from_card` in `locales/de.json` and `en.json`
  (`cargo test -p quantus-texts`). Test: the applet cannot close or change the menu; *Beenden*
  leaves as AC does.
- [x] **1.6 Touch.** `render::target`: the left of the bar is `Target::Back` (leaves), a soft
  label is `Target::Soft`; a tap inside the area becomes `Applet::tap` with the point in the
  area's coordinates. Test in `ui/tests/apps.rs` beside the keys.

Done when `cargo test -p quantus-ui --test apps` passes and the other tests of `quantus-ui` that
touch `Screen` (`screen`, `menus`, `sheets`, `touch`, `soft_keys`) still do.

### Step 2. hui: page 3 with the Apps and Setup tiles (M)

- [x] **2.1 The tiles.** `Mode::Apps` and `Mode::Setup` at the end of `Mode::ALL`
  (`[Mode; 14]`), their texts (`hui.app.apps`, `hui.app.setup`), two more colours in
  `colors::TILES` (F6). `Mode::engine` and `Mode::sheet` say no for both; `core_mode` in
  `firmware/engine/src/lib.rs` gets its arms. `App::choose`: *Setup* opens
  `Menu::new(&tree::SETUP)` as SHIFT + MODE does and leaves `self.mode`. Check the tests that
  count tiles or pages (`ui/tests/menus.rs`, `touch.rs`, `tour.rs`) and the tour itself. Test:
  ↓ from tile 10 reaches page 3; the third dot shows; *Setup* in Prüfungsmodus opens SETUP.
- [x] **2.2 The list.** `Backend::apps(lang) -> Vec<AppInfo>` and
  `Backend::open_app(id) -> Result<Box<dyn Applet>, String>`, both with defaults.
  `Edit::Apps { label, apps, selected }` in [`menu.rs`](../ui/src/menu.rs), filled at runtime as
  `Edit::Sessions` is, with `Outcome::OpenApp(usize)`; `list_len` and `menu_screen` in the
  renderer get their arms. Two groups, *Spiele* and *Werkzeuge*, built in first; each line the
  name and the author; a digit picks, OK opens, ← or AC goes back. An app whose `api` is newer
  is grey and says so when picked. Without apps from the card, the line *Eigene Apps: Ordner
  /apps auf der SD-Karte*. Texts under `hui.apps.*`. Test with a backend that lists two apps
  and opens the Rust applet of step 1.
- [x] **2.3 OPTN on a line, and errors.** OPTN on an app in the list opens *Info* and *Daten
  löschen* without starting it. An `Err` from `open_app` is shown as `Edit::Note` and OK goes
  back to the list. Test both.
- [x] **2.4 Prüfungsmodus.** With `Settings::exam` the Apps tile opens a message
  (`hui.apps.exam`) and nothing else; a running app is left when the exam mode is switched on
  (it can only be switched in SETUP, which is not reachable from inside an app, so this is one
  assertion). Test.

M1: `cargo run -- --keys '…' --png out.png` in `firmware/sim` shows page 3 and the empty list.

### Step 3a. hui: the templates the games need (M)

The template's state lives in `Running`, beside the applet; `Applet::input()` says which one
holds. The hui handles the template's keys first and passes the rest on (apps.md §4.4).

- [x] **3a.1 `Input` and `"keys"`.** `enum Input { Keys, Grid {..}, List {..}, Line {..},
  Text {..}, Cells {..} }` in `applet.rs`, and `enum Event { Entered(Value), Pick(col, row) }`
  given to a new `Applet::event`. `"keys"` is what step 1 does. The layout helper
  `Layout::grid(area, cols, rows)` with `cell`, `at` and `side` (apps.md §4.5) is here, in the
  hui, because the grid template and the scripts must agree on the cells. Unit tests of the
  layout: 480 × 250 puts `side` to the right, 320 × 214 below. Built as `applet::Grid`, and
  `Event` as `Pick`, `Chosen`, `Entered` (text or LaTeX) and `Cell`, no script value in the hui.
- [x] **3a.2 `"grid"`.** The arrows move the cursor, with `wrap` round the edges; OK or EXE
  gives `Event::Pick`; `digits` maps 1–9 to a 3 × 3 grid as the number block lies (7 top left);
  the hui draws the cursor's frame over the applet's shapes; a tap picks the cell under it.
  `Host::cursor` and `set_cursor`. Test: a Rust applet with a 3 × 3 grid gets the picks; keys
  the grid does not use still reach `key`.
- [ ] **3a.3 `"list"`.** `options.items` drawn as a menu list in the area (reuse `line_rect` and
  the list drawing of `screens.rs`), ↑/↓, a digit, OK; `Event::Entered(index)`. Test.
- [ ] **3a.4 `confirm`.** A question of the hui's over the app with *Ja* and *Nein*, AC is
  *Nein*; while it is open the applet gets nothing. On the hui's side it is
  `Running::question: Option<Question>`, opened by a request from the applet and answered into
  it (`Applet::answer`), which is what the script thread of F1 waits for. Test.

### Step 4. `firmware/apps`: the host (L)

The crate `quantus-apps`, a member of the main workspace, depending on `quantus-ui` and the
interpreter. No engine.

- [ ] **4.1 The crate and the header.** `header::parse(text) -> Option<Header>`: the comment
  lines at the top, `name`, `name.<lang>`, `kind`, `api`, repeated `author`, `version`, `about`,
  `about.<lang>`, `license`, each at most 80 characters; no `name` gives `None`. `API: u32 = 1`.
  `fn info(id, text, built_in, lang) -> Option<AppInfo>`. Tests on header texts, among them a
  file with a BOM, with `\r\n`, with a header after the first line of code (ignored).
- [ ] **4.2 The script thread.** `Script::start(source) -> Result<Script, Error>`: compiles on
  the new thread, runs `init`. The main side has `send(Callback) -> Yield` with
  `Yield::Done(Frame)`, `Yield::Asks(Request)` and `Yield::Failed(Error)`, and
  `answer(Value) -> Yield`. It waits with a timeout; over it the app counts as hung. Dropping
  `Script` closes the channel and the thread ends; a thread still inside a callback is left to
  the operation budget. `Error` carries the message and the line. Tests: a callback returns; a
  host call yields and resumes; a script error has its line; a dropped script's thread is gone.
- [ ] **4.3 The limits.** The operation budget per callback (the number from 0.4), call depth,
  sizes of strings, arrays and maps, at most 256 KB of source, a few thousand shapes per frame.
  Tests: each limit as its own script, each an `Error`, none a panic; the texts *Die App
  antwortet nicht* and the error page's (`hui.apps.not_responding`, `.error`, `.line`).
- [ ] **4.4 Callbacks, keys and drawing.** `ScriptApplet: Applet` over `Script`: `key` turns
  `AppKey` into its name (§4.2) and calls `key(k)`, then `draw()`; `tick` calls `tick(ms)` when
  the timer set with `timer(ms)` is due (at least 100 ms). The drawing functions of §4.3
  collect plain draw commands on the script's side, which the applet replays into the `Canvas`
  in `Applet::draw`; `width()`, `height()`, `text_width` answer from the area of the last frame
  and the hui's font metrics. Colours by name or `rgb`. An error at any point turns the applet
  into the error page (message, line; OK or AC go back). Tests run the Counter of apps.md §4.1
  and compare the shapes.
- [ ] **4.5 `soft`, `input`, the helpers.** `soft([...])`, `input(kind, options)` for the
  templates of 3a with `pick` and `entered`, `cursor`, `confirm` (a yield, F1), `grid`, `side`,
  `random(n)` (through `Host::random`), `timer`, `lang`, `exit`. Tests: one small script per
  function.
- [ ] **4.6 Into the simulator and the board.** `quantus_apps::Apps { card: Option<PathBuf> }`
  with `list(lang)` and `open(id)`; the built-in apps from `firmware/apps/apps/*.rhai` with
  `include_str!` (one test script for now). `Sim` and `Board` hold one and implement `apps`,
  `open_app` and `random` (F3): `esp_random` on the board, in the simulator a small generator
  that `--seed <n>` fixes (F11). The dependency goes into `sim` and `handheld`; all three lock
  files change. Check: the test script opens from the Apps list in the simulator, and on the
  board over the serial keys.

### Step 5. Tic tac toe (M)

- [ ] **5.1 The script.** `firmware/apps/apps/tictactoe.rhai` as apps.md §6.1: `grid(3, 3)`,
  the `"grid"` template with `digits`, `side()` for the turn and the score, the soft keys *Neu*,
  *2 Spieler* / *Computer*, *Stufe*; the winning line marked; any key starts the next round
  with the other side. German and English through `lang()`.
- [ ] **5.2 The computer.** *Leicht* picks a free cell with `random`; *schwer* searches the game
  tree. Measure the first move of the search against the operation budget on the board; if it
  does not fit, the first move comes from a table (centre, else a corner) and the search starts
  with the second.
- [ ] **5.3 Tests and a look.** `firmware/apps/tests/tictactoe.rs`: a game through keys against
  the seeded computer; *schwer* loses none of a few hundred seeded games against random moves;
  the layout at 480 × 250 and at 320 × 214 keeps board and side panel inside the area. PNGs
  from the simulator, then the board: the bands sent per move (`main.rs` logs them).

M2.

### Step 6. 2048, with saved data (M)

- [ ] **6.1 `save` and `load`.** In `quantus-apps`: a script value to JSON and back (maps,
  arrays, numbers, strings, booleans, `()`), at most 64 KB; `Host::load` and `Host::save`.
  `Backend::app_data(id)` and `store_app_data(id, json)` with defaults; the hui's `Host` keeps
  the data in memory when the backend keeps none. Tests: a round trip of nested values; over
  64 KB is an error of the script, not a cut file.
- [ ] **6.2 The files.** `Apps::data` and `store_data` on `/apps/data/<id>.json`, written to a
  temporary name and renamed, as far as FAT allows; an id is checked before it becomes a path
  (letters, digits, `-`, `_`). In the simulator not with `--png` (F11). `leave()` is called on
  AC, MODE, *Beenden* and `exit`, and before *Neu starten*. *Daten löschen* in the app's menu
  and from the list removes the file. Tests with a temporary folder.
- [ ] **6.3 The script.** `firmware/apps/apps/2048.rhai` as apps.md §6.2: `"keys"`, the slide
  and merge, a 2 nine times in ten, the new tile outlined, colours from the tile palette, the
  small font from four digits on, *Neu* with `confirm`, *Zurück* once, 2048 reached, the end.
  It saves after every move, so switching off loses nothing.
- [ ] **6.4 Tests and a look.** `firmware/apps/tests/g2048.rs`: slides and merges from set
  boards (`[2,2,2,2]` gives `[4,4,0,0]`), a move that changes nothing adds no tile, a game
  restored after reopening, the best score kept across *Neu*, both sizes. On the board: the
  redraw time of a move, written into apps.md §6.2 for the later animation.
- [ ] **6.5 Tic tac toe saves** its mode, level and score.

M3: the games are done.

### Step 7. Apps from the card (M)

- [ ] **7.1 Listing.** `Apps::list` reads `/apps/*.rhai`: the first 2 KB of each file for the
  header, the size limit, no `name` or another ending left out, a newer `api` listed as not
  usable; an id that a built-in app has is listed beside it, marked as the card's (the id
  becomes `card:<id>` inside, so the data do not mix). Sorted by name within the groups. Tests
  with a folder of good and bad files in `firmware/apps/tests/card/`.
- [ ] **7.2 The simulator and the board.** `firmware/sim/sd/apps/` with one example tool (the
  Counter); `sd::APPS` in `handheld/src/sd.rs`. The list is read each time the Apps tile opens,
  so a card changed in between shows. Measure the listing of twenty files on the board (the
  card runs at 10 MHz).
- [ ] **7.3 Storage.** SETUP → Gerät → Speicherplatz counts `/apps/` (`Storage`, `sd::storage`,
  the simulator's `storage`).

M4.

### Step 3b. hui: the templates for tools (L)

- [ ] **3b.1 `"text"` and `ask`.** One line of plain text at the bottom of the area with a
  label, the typing of `Edit::Text` reused (letters, digits, space, DEL; `max`, `value`);
  `ask` as a question over the app, answered like `confirm`; AC gives `()`. The area is lower
  while the line shows. Tests with a Rust applet, then `input("text", ..)` and `ask` in
  `quantus-apps`.
- [ ] **3b.2 `"line"`.** An `Editor` of its own in `Running`, with the modifier layers, SYMB
  and the keys `calc_key` handles for typing; what `calc_key` does to the editor is first moved
  into a function both use, in a commit of its own that changes no behaviour (`ui/tests/calc.rs`
  and `editor.rs` stay green). The soft keys while the line has the cursor as F9 says. EXE
  gives `Event::Entered` with the LaTeX. Tests: a fraction with a unit typed into an app's line.
- [ ] **3b.3 `"cells"`.** The grid of 3a.2 with the cell editor of `sheet::Sheet` in the cell
  under the cursor; `digits_only` takes one digit and moves on. `Event::Entered` with column,
  row and value. Test.

### Step 8. The calculator from a script (M)

- [ ] **8.1 `calc` in the engine.** `Backend::app_calc(mode, settings, input) -> Option<..>`
  started and polled as `start` and `poll` are, on `Engine::calculate(.., record: false)` (F4),
  through the worker on the board (`Job::Calculate` with `record: false` exists). The answer as
  `#{ok, value, latex, error}`: `latex` as the calculator shows it, `value` a text `calc` takes
  again. The mode is the one that was open when the app started.
- [ ] **8.2 `calc` in the host.** A yield (F1): the hui sets `busy`, the spinner turns, the
  answer resumes the script; AC drops the calculation (`drop_pending`) and leaves. Tests in
  `firmware/engine/tests/apps.rs` against the real engine: a script that adds two lengths, an
  error from the engine as `ok: false`, the history unchanged.
- [ ] **8.3 `var`, `insert`, `now`.** `var(name)` from `Backend::variables(List)`; `insert`
  leaves the app and puts the LaTeX at the cursor of the calculation (`Applet::leave` gives it,
  `App::insert` takes it); `now()` from `clock::now`. Tests.
- [ ] **8.4 `calc: true` on the `"line"` template.**

### Step 9. Forms (L)

- [ ] **9.1 The description.** `struct Form` in the hui, free of anything only the hui knows
  (apps.md §3): title, fields (id, label, unit, value, hint, choices), results (label, formula,
  unit). `quantus-apps` reads it from the script's map and reports what is wrong with the
  field's name. Tests on good and bad descriptions.
- [ ] **9.2 The form applet.** `FormApplet: Applet` in the hui on the sheets' fields
  ([`sheet.rs`](../ui/src/sheet.rs), [`render/sheet.rs`](../ui/src/render/sheet.rs)): one field
  per line with label and unit, ↑/↓, the editor in the field, `choices` as a list, `hint` below.
  More fields than fit scroll. Test with a form built in Rust.
- [ ] **9.3 Working it out.** EXE on the last field: each field as `value` with its unit if it
  has none, substituted into each `formula` (F4), worked out through `app_calc`, converted to
  the result's `unit`; an error of the engine lands on the field it names where that can be
  told, else under the results. The results as an answer below; OK on one hands it to the
  calculation. Tests against the real engine: the voltage divider of apps.md §4.7, `10 kΩ`
  typed into a field in Ω, a length typed into a voltage.
- [ ] **9.4 `form` from a script.** `form(description)` as a yield that gives the results when
  the form closes; a script that is only a `form(...)` call is a tool with no callbacks. With
  `submit` the function is called with the values and gives results or a field's error. The
  inputs are saved with the app's data and stand in the fields next time. Tests.
- [ ] **9.5 An example on the card.** `firmware/sim/sd/apps/spannungsteiler.rhai`.

M5.

### Step 10. Search and touch (S)

- [ ] **10.1 Apps in CATALOG.** `search::Target::App(id)`; `Index::new` takes the list of apps
  and is built again when it or `Settings::exam` changes (F10); a hit opens the app. In
  Prüfungsmodus apps are not in the index. Test in `ui/tests/search.rs`.
- [ ] **10.2 `tap` and `at`.** `tap(x, y)` reaches the script; `grid(..).at(x, y)` gives the
  cell. Tic tac toe and 2048 need nothing (the grid template takes taps; 2048 gets a swipe as
  an arrow if `App::touch` can tell one). Test.

### Step 11. Documentation (M)

- [ ] **11.1 For app authors.** A reference page of the API in `docs/docs/` (the handheld's
  part of the site), with the Counter, the two games' sources linked, a form, the limits and
  how to pass an app on.
- [ ] **11.2 The firmware's pages.** `/apps/` and `/apps/data/` in [sd-card.md](sd-card.md),
  the lines in [features.md](features.md), the tiles and page 3 in
  [handheld-menus.md](../../design/handheld-menus.md) §7.1, the `--seed` option and `sd/apps/`
  in the simulator's README.
- [ ] **11.3 `CHANGELOG.md`**, under `## [Unreleased]`.
- [ ] **11.4 `THIRD-PARTY-LICENSES.md`** for the interpreter and what it brings (F12).
- [ ] **11.5 apps.md** from plan to description: the state line, §8 removed, plans.md §7 removed.

M6.

## 4. Risks

| Risk | Seen in | What then |
|---|---|---|
| Rhai is too slow, too large or needs too much stack | 0.2, 0.3 | Lua (`mlua`); the API of apps.md §4 carries over, steps 1 to 3 are untouched because the hui never sees the interpreter |
| The script thread's stack does not fit into internal RAM beside the engine's 64 KB | 0.3 | The stack from PSRAM, or the script on the engine's worker thread while no calculation runs |
| A panic inside the interpreter halts the board | 0.4, 4.3 | Every limit has a test that would abort; a crash found later is a bug in the limits, not something to catch |
| *schwer* in tic tac toe is over the operation budget | 5.2 | A table for the first move; a larger budget for built-in apps is not wanted, the games are the examples |
| Writing `/apps/data/` after every move is slow or wears on the card | 6.4 | Save on `leave` and every few moves instead |
| `"line"` needs more of `calc_key` than can be shared cleanly | 3b.2 | The line without the unit bar first; tools can still take units through UNIT |
| The form's substitution of values into formulas goes wrong for names like `R1` inside `R12` | 9.3 | Substitute on the parsed input, not on text; if the core has to help, it is a function in `quantus-core` and the desktop gets it too (ideas 5 and 19) |

## 5. Open, to be decided on the way

- **F1 against the callback style**, in 0.5, with the spike's numbers.
- **The soft keys while an app's input line has the cursor** (F9), in 3b.2.
- **Long calculations from a script** (apps.md §10): 8.2 gives the spinner and AC; whether more
  is needed shows with the first real tools.
