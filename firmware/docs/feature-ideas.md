# Feature ideas

Ideas that are **not planned yet** and would make Quantus more useful in daily use, on the
handheld and on the desktop. None of them is in [plans.md](../../plans/plans.md),
[handheld-menus.md](../../design/handheld-menus.md), [physics-and-circuits.md](../../plans/physics-and-circuits.md)
or [new-areas-and-themes.md](../../plans/new-areas-and-themes.md); where one builds on a planned item,
it says so. What the handheld has today is in [features.md](features.md).

Each idea says what it is, why it helps, how it could work, where the work goes (**core** is
`quantus-core`, **hui** the handheld UI, **desktop** the desktop app), a rough effort (S: days,
M: one or two weeks, L: more) and what is still open. The rule from handheld-menus.md §8 holds:
maths, data and state go into the core, so both shells get them.

State on 2026-11-02. An idea that is taken up moves into plans.md and out of this file.

## Contents

1. Working faster: [search everywhere](#1-search-everywhere), [user-defined functions](#2-user-defined-functions),
   [history search and pins](#3-history-search-pins-and-favourites), [result references](#4-references-to-earlier-results),
   [macros](#5-macros-and-small-programs)
2. Understanding: [help key](#6-a-help-key), [errors that point](#7-errors-that-point-at-the-input),
   [practice mode](#8-practice-mode), [notes](#9-notes-on-calculations)
3. More maths for everyday tasks: [graph screen](#10-a-graph-screen-on-the-handheld),
   [uncertainties](#11-measurement-uncertainties), [dates and durations](#12-dates-and-durations),
   [money](#13-percent-and-money), [programmer view](#14-a-programmer-view-for-basis-n)
4. Getting data in and out: [export](#15-export-a-session), [screenshots](#16-screenshots),
   [USB drive](#17-the-sd-card-as-a-usb-drive), [clipboard](#18-a-shared-clipboard-with-the-desktop),
   [packs on the card](#19-constant-unit-and-formula-packs-on-the-card)
5. The device: [resume where I was](#20-resume-where-i-was), [battery details](#21-battery-details),
   [readability](#22-light-theme-high-contrast-and-large-print), [key feedback](#23-key-feedback),
   [exam mode for teachers](#24-an-exam-mode-teachers-can-trust), [classroom](#25-worksheets-and-sharing-in-class)

---

## 1. Working faster

### 1. Search everywhere

**What.** One key (or SHIFT+OPTN) opens a search line. Typing a few letters lists matching
functions, units, constants, settings, modes and menu entries together: `ohm` finds the unit Ω
and the constant R_K; `wurz` finds √ and ⁿ√; `zeit` finds Uhrzeit, Zeitzone and the unit group
Zeit.

**Why.** The OPTN, CONST and CONV trees are deep, and with 68–77 keys many functions sit two or
three levels down. A user who knows the name should not need to know where it lives. On the
desktop the constants and units picker would get the same search.

**How.** The core exports one index: every function, unit, prefix and constant with its names in
German and English, aliases (`wurzel`, `sqrt`, `root`), and the LaTeX to insert. The hui adds its
menu entries from `tree.rs`. Matching by prefix and by words inside the name, ranked by exact
match, then by how often the entry was used (a small count kept in NVS). Choosing an entry inserts
it, opens the setting, or jumps to the menu page. Typing needs ALPHA letters, so this waits for
the keypad legend; until then it works on a USB keyboard.

**Where.** Core: the index. hui: the search screen. Desktop: search in the picker and a command
palette (Ctrl+K).

**Effort.** M.

**Open.** Which key opens it; whether recent entries show before anything is typed.

### 2. User-defined functions

**What.** `f(x) := x^2 - 3x` defines a function in the session; `f(2)`, `f'(x)`, `\int_0^1 f(x)\,dx`
and `f(a+1)` use it in every mode. Kurvendiskussion and Wertetabelle offer the defined functions
instead of retyping them.

**Why.** Session variables exist, but a formula that is used again and again with other values
(a cost function, a physics law with one free variable) has to be typed again each time or
re-run through CALC. Functions are what a student writes down on paper anyway.

**How.** In the core, a function is a session entry with parameters and a body; substitution
reuses what CALC does. Definitions are kept in the session JSON (a new format version). VARS on
the handheld lists them next to the variables, with edit and delete. Recursion is refused.

**Where.** Core: parser (`:=` with a parameter list), session, evaluator. hui: VARS, and a
picker in Kurvendiskussion and Wertetabelle. Desktop: the variables panel.

**Effort.** M.

**Open.** Several parameters (`g(x; y)`); units on parameters in Physik.

### 3. History search, pins and favourites

**What.** In the history, a search over inputs and results; a pin that keeps an entry at the top
of the session; a favourites list across sessions for inputs used often (a formula from a
recurring exercise, a unit conversion done daily).

**Why.** The history is 50 entries per session, browsed with ↑/↓ one by one. After a long
session the one calculation that mattered is hard to find again.

**How.** Pins are a flag on a history entry in the session. Favourites are a small file of their
own on the card (`/favourites.json`) and in the desktop's config directory, with an input, its
mode and an optional name. The hui shows them as a menu like Sitzungen; choosing one puts the
input in the editor.

**Where.** Core: the flag and the favourites format. hui and desktop: the screens.

**Effort.** S–M.

**Open.** Whether favourites sync with the desktop along with the sessions.

### 4. References to earlier results

**What.** Beyond Ans and PreAns: any earlier result by its number in the history (`#3`, shown as
a small chip in the input), and a key that moves the cursor into the history to pick one and
insert it.

**Why.** Real tasks chain three or four intermediate results; Ans only holds the last one, and
STO needs a name for every step.

**How.** The core resolves `#n` against the session's history when it evaluates; the hui picks
one with ↑/↓ and OK while the editor is open. A reference stays valid when the session is
reopened, because the history is in the session.

**Where.** Core: parser and session. hui: the picker. Desktop: click on a history result inserts
its reference.

**Effort.** S.

**Open.** What happens to `#n` when an entry is deleted (renumbering vs. a stable id).

### 5. Macros and small programs

**What.** Record a sequence of calculations (with inputs asked for, like CALC) and replay it with
new values. Example: a lab evaluation that computes a mean, a deviation and a result in a unit,
run once per measurement series.

**Why.** Graphing calculators sell on programs; a simple recorded sequence covers most of what
students and technicians use them for, without a programming language.

**How.** A macro is a list of inputs with free variables, saved like a session. Replaying asks
for each free variable once and runs the inputs in order, filling the history. Later, if wanted:
a condition and a loop. Kept in `/macros/` on the card.

**Where.** Core: the macro format and the replay through the API. hui: record, list, run.
Desktop: the same, with an editor.

**Effort.** M (recorded sequences), L (with conditions and loops).

**Open.** Whether macros are allowed in Prüfungsmodus (probably not).

## 2. Understanding

### 6. A help key

**What.** A help key followed by any key or a highlighted
menu entry shows what it does: one sentence, the syntax and one example with its result. In a
SETUP entry it explains the setting and its effect with an example ("Nenner: faktorisiert —
`\frac{1}{x^2-1}` stays `\frac{1}{(x-1)(x+1)}`").

**Why.** Settings like *Identitäten mit Bedingung* or *Buchstabenfolge als Produkt* are not
self-explaining, and there is no manual on the device.

**How.** The texts live in the core next to the settings and functions, in German and English, so
the desktop shows the same text as a tooltip. The examples are computed, not typed, so they stay
right; a test runs every example.

**Where.** Core: the texts and examples. hui: the help screen. Desktop: tooltips.

**Effort.** M (mostly writing).

**Open.** Whether help covers the modes too (a short "what this mode is for" page).

### 7. Errors that point at the input

**What.** An error marks the part of the input it is about (the box in the editor turns red) and
says what to do: "Division durch 0 in diesem Nenner", "Einheiten passen nicht: m + s", "Hier fehlt
ein Argument".

**Why.** An error message alone on a small screen makes the user search the whole input; a
pointer saves that, especially in long Physik inputs with units.

**How.** The core's errors carry the span of the LaTeX they refer to; the hui maps the span back
to the editor's tree (it wrote the LaTeX, so it knows which node produced which range). The
desktop's MathLive can select the same range.

**Where.** Core: spans on errors. hui and desktop: the marking.

**Effort.** M.

**Open.** How fine the spans can be for errors found late (during simplification).

### 8. Practice mode

**What.** The user types a task and their own answer; Quantus says right or wrong, and on request
shows the next step of the working as a hint, then the next, before the full solution.

**Why.** The step list (Rechenweg) already exists; showing it all at once tempts copying. Step by
step, the calculator becomes a tutor, which suits students at home.

**How.** Builds on Prüfen (equality check) and the step list: the answer is compared with the
result as an equality in the core (so `\frac{1}{2}` and `0.5` both count, with a setting for
whether a decimal counts as exact). Hints are the steps at the chosen detail level, one per key
press.

**Where.** Core: an answer check that accepts equivalent forms. hui and desktop: the mode.

**Effort.** M, after the step list is on the handheld.

**Open.** Whether equivalent but unsimplified answers count as right.

### 9. Notes on calculations

**What.** A short text note on a history entry or on a session ("Aufgabe 3b", "Messreihe 2,
Raum 104").

**Why.** Sessions get dates as names; a note makes a session or a result findable later and
explains it when it is exported.

**How.** A text field in the session JSON per entry and per session. Typed on a USB keyboard or
with ALPHA. Shown in the history in a small font, and in exports.

**Where.** Core: session format. hui and desktop: entry and display.

**Effort.** S.

**Open.** None worth noting.

## 3. More maths for everyday tasks

### 10. A graph screen on the handheld

**What.** A graph of f(x) on the full screen, with the extrema, zeros and inflection points from
Kurvendiskussion marked; zoom (+/−) and pan (arrows, drag); a trace cursor that walks along the
curve and shows x and f(x); several functions at once, each in a tile colour.

**Why.** The planned Kurvendiskussion screens are text, section by section. A graph is what a
student checks first and what makes the results believable; every graphing calculator has it.
The desktop already draws one (`FunctionPlot`).

**How.** The core's `plot` already samples the function and picks a window (`analysis/plot.rs`),
so the hui only draws the segments with embedded-graphics, in bands like everything else. Zoom
asks the core for a new sample in the new window. Touch: pinch is not possible on a resistive
panel, so zoom stays on keys, pan by dragging.

**Where.** Core: nothing new for one function; a cheaper resample for zooming. hui: the graph
screen, reached from Kurvendiskussion and Wertetabelle.

**Effort.** M.

**Open.** How long a resample takes on the P4; whether parametric and polar graphs follow.

### 11. Measurement uncertainties

**What.** Values with an uncertainty in Physik: `(9{,}81 \pm 0{,}02)\,\mathrm{m/s^2}`, carried
through the calculation (Gaussian propagation) and rounded in the result as a lab report wants
it: the uncertainty to one or two significant digits, the value to the same place.

**Why.** Every school and university physics lab needs this, and today it is done by hand with
partial derivatives. Quantus already has units, exact derivatives and steps, which is most of
what the propagation needs.

**How.** A value type in the core with a mean and a standard uncertainty, propagated linearly
through the derivative of each operation. The step list shows the propagation formula with the
partial derivatives. A setting picks Gaussian or maximum error.

**Where.** Core: the value type, parser (`\pm`), formatter, steps. hui: a ± key or OPTN entry.
Desktop: the same in the syntax.

**Effort.** L.

**Open.** Correlated quantities (probably out of scope); relative uncertainties as input (`± 2 %`).

### 12. Dates and durations

**What.** Days between two dates, a date plus a number of days or weeks, the weekday of a date,
times added (`1\,\mathrm{h}\,45\,\mathrm{min} + 2\,\mathrm{h}\,30\,\mathrm{min}`) and shown as
h:min.

**Why.** Everyday questions (deadlines, working hours, age in days) that a calculator is asked
and today cannot answer. The handheld already has a calendar in `clock.rs` and a clock.

**How.** A date literal in the core (`2026-11-02`), dates minus dates give days, dates plus
durations give dates, using the core's units for durations. A result form h:min:s for times,
next to °′″.

**Where.** Core: date type, calendar (move the one from `firmware/ui/src/clock.rs` into the
core), result form. hui: input template or OPTN entry, "today" from the clock.

**Effort.** M.

**Open.** Time zones in date arithmetic (probably none: dates are local).

### 13. Percent and money

**What.** Percent changes (`+19 %`, "what % is 30 of 120"), VAT in and out, compound interest,
loans and annuities (the time value of money: present value, rate, periods, payment, future
value), and a currency unit with rates fetched over WLAN on request.

**Why.** Most daily calculator use outside school is money. The Casio fx-991 has none of it,
which is a gap Quantus can fill.

**How.** Percent arithmetic in the core (the `%` unit exists). A TVM solver: one equation in
five quantities, solved for the one left empty; it fits the SOLVE screen. Currencies as units
whose factor comes from a rate table the shell sets; the hui fetches it over WLAN (never in
Prüfungsmodus) and keeps the date it was fetched, shown with every result.

**Where.** Core: percent operations, TVM, currency units with external rates. hui: an OPTN
group, the rate download. Desktop: the same, rates over the internet.

**Effort.** M (percent and TVM), M (currencies).

**Open.** Which rate source (free, without a key); rounding rules for money.

### 14. A programmer view for Basis-N

**What.** In Basis-N, a view of the result's bits in groups of four, the word size (8, 16, 32,
64 bit) and signed or unsigned; shifts and rotations; an IEEE-754 view that shows a float's sign,
exponent and mantissa.

**Why.** Basis-N on the Casio is fixed at 32 bit; programmers and electronics students need
other widths and the bit picture, and the electronics mode is in Quantus already.

**How.** Word size and signedness as Basis-N settings in the core; the bit view is a result form
the hui draws. Shifts and rotations as functions.

**Where.** Core: settings, operations, result form. hui: the view. Desktop: the same in its
Basis-N mode.

**Effort.** S–M, once Basis-N is on the handheld.

**Open.** Whether the IEEE view belongs here or in a separate tool.

## 4. Getting data in and out

### 15. Export a session

**What.** A session written to the SD card as Markdown, LaTeX (`align*`, as the desktop copies),
or PDF: each input, its result and, if wanted, the steps and notes.

**Why.** Homework and lab reports need the working on paper or in a document; today the session
JSON is only readable by Quantus.

**How.** The core already writes LaTeX for results and the desktop's writeup. A core function
turns a session into Markdown or a LaTeX document; the hui saves it to `/export/`. PDF on the
desktop only (it has a renderer); on the handheld LaTeX and Markdown are enough.

**Where.** Core: the export. hui: an entry in Sitzungen. Desktop: export in the session menu.

**Effort.** S–M.

**Open.** Whether plain text is wanted as a third format.

### 16. Screenshots

**What.** A key combination saves the screen as a BMP (or PNG) to `/screenshots/` on the card.

**Why.** For questions to a teacher, bug reports, the documentation and the website.

**How.** The renderer draws in bands; a screenshot draws all bands once more into a file instead
of the display. No frame buffer is needed.

**Where.** hui and board.

**Effort.** S.

**Open.** Which keys.

### 17. The SD card as a USB drive

**What.** While on USB, the handheld offers its SD card as a USB drive, so wallpapers, sessions,
exports and screenshots can be copied without opening the case or removing the card.

**Why.** The card slot is meant to be reachable, but taking the card out is fiddly and loses it
for the calculator meanwhile.

**How.** TinyUSB's mass storage class on the P4's USB OTG port; the calculator stops using the
card while it is mounted, with a screen that says so.

**Where.** Board firmware.

**Effort.** M.

**Open.** Whether the USB-C port can be a device for this and the serial console at once (it is
also the flashing port).

### 18. A shared clipboard with the desktop

**What.** "Send to desktop" on a result puts its LaTeX on the desktop's clipboard; the desktop can
send an input to the handheld's editor.

**Why.** The quickest way from a calculation on the device into a document on the PC, and from a
task on the PC onto the device.

**How.** One more message in the planned link between handheld and desktop (plans.md §5), over
WLAN or Bluetooth with the same pairing.

**Where.** hui and desktop, on the link.

**Effort.** S, once the link exists.

**Open.** None beyond the link itself.

### 19. Constant, unit and formula packs on the card

**What.** Files in `/packs/` that add constants, units or formulas: a chemistry pack (molar
masses of the elements), an electrical pack (wire gauges, standard resistor series E12/E24), a
course's own constants.

**Why.** No built-in list covers every subject, and the formula collection planned for the
desktop (physics-and-circuits.md §1) would grow without end; packs keep the core small and let
teachers add their own.

**How.** The same data format the formula collection plans, plus one for constants and units;
the core loads them through the API at start, checks their dimensions, and refuses a pack with
an error. The hui lists packs in SETUP to switch them on or off.

**Where.** Core: loading and checking. hui and desktop: the list.

**Effort.** M.

**Open.** Packs in Prüfungsmodus (likely off unless the teacher allows them, see idea 24).

## 5. The device

### 20. Resume where I was

**What.** After switching off and on, the screen comes back exactly as it was: the mode, the
input with the cursor in it, the open menu, the result shown.

**Why.** Switching off is a cold boot and the session is saved, but the input being typed and
the place in the menus are lost. A calculator is switched off between tasks all the time.

**How.** At power-off (the latch's INT) the hui writes a small state file to the card or NVS: the
editor tree as LaTeX plus the cursor position, the mode, the menu path. At boot it is read back
after the first frame.

**Where.** hui and board.

**Effort.** S.

**Open.** Whether a half-typed input should survive a forced power-off (it cannot, nothing is
saved then).

### 21. Battery details

**What.** A SETUP page with the charge in percent, the time left at the current use, the cell's
voltage and health, and the charging state; a warning at 10 %.

**Why.** A battery icon alone does not tell whether the device lasts the exam.

**How.** The MAX17048 gives charge and rate; the estimate uses the average current of the last
minutes. A warning screen once, at 10 %, before the firmware saves and switches off at the
low-voltage threshold.

**Where.** Board and hui.

**Effort.** S, once the gauge is on the PCB.

**Open.** None.

### 22. Light theme, high contrast and large print

**What.** A light palette for daylight, a high-contrast palette, and a large-print mode that
shows the input and result in a bigger font with fewer lines; tile colours chosen so they stay
apart for colour-blind users.

**Why.** A backlit dark screen is hard to read outdoors and for people with low vision. The
desktop plans light and dark themes (new-areas-and-themes.md); the handheld has only the purple
dark palette.

**How.** The hui's colours are already named constants (`render::colors`); a palette becomes a
table chosen in SETUP → Gerät. Large print needs a second font size in the layout, which the
planned Schriftgröße setting starts.

**Where.** hui.

**Effort.** S (palettes), M (large print).

**Open.** Whether wallpapers are allowed with the light palette.

### 23. Key feedback

**What.** An optional short click from a small buzzer or a vibration motor on each key press,
and a different sound for an error.

**Why.** Low-profile SMD switches under printed caps have little travel; feedback avoids double
presses and missed keys.

**How.** A piezo buzzer or a small ERM motor on a GPIO with a transistor; off by default, a
SETUP entry. Silent in Prüfungsmodus.

**Where.** Hardware (one part on the PCB) and board firmware.

**Effort.** S, but it must go on the PCB before it is made.

**Open.** Buzzer or motor, and the current it draws.

### 24. An exam mode teachers can trust

**What.** Prüfungsmodus that can be seen and checked: a status LED that blinks while it is on, a
log of when it was switched on and off (with the time), and an exam profile chosen by the teacher
that locks modes, packs and macros, loaded from a QR code or a file.

**Why.** Today Prüfungsmodus switches the radios off, but a teacher cannot see that from across a
room, and exams differ in what they allow (no CAS, no steps, no stored formulas). Calculators
admitted in German exams need a checkable exam state.

**How.** An LED on the PCB driven only by the firmware in exam mode. The log is kept in NVS and
shown on a page that cannot be cleared while the mode is on. A profile lists the allowed modes
and functions; the hui hides the rest and the core refuses them. Leaving the mode clears the
sessions made during it, or keeps them, as the profile says.

**Where.** Hardware (LED), board, hui, core (refusing functions by profile).

**Effort.** M–L.

**Open.** What the German exam rules (for example the Abitur's) require of a calculator; whether
a profile should need a code to leave.

### 25. Worksheets and sharing in class

**What.** A teacher puts a worksheet (a list of tasks, as inputs or text) on the card or sends it
as a QR code; students open it as a session and work through it. Results can be shared back as a
QR code of the session.

**Why.** Brings the device into lessons without a network, which many schools do not allow.

**How.** A worksheet is a session file with tasks and no results. The QR code of a session uses
the planned QR code of the result (handheld-menus.md §5.5), with a compact encoding and a short
reader page on the desktop or the website.

**Where.** Core: the worksheet format. hui: open and work through. Desktop: create worksheets,
read QR codes.

**Effort.** M.

**Open.** How much fits in one QR code; a QR reader on the desktop needs a camera.
