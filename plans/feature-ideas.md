# Feature ideas

Ideas that are **not planned yet**, for the handheld and the desktop: new modes, features in
existing modes, and what would make Quantus more useful in daily use. None of them is in
[plans.md](plans.md), [handheld-menus.md](../design/handheld-menus.md),
[physics-and-circuits.md](physics-and-circuits.md) or
[new-areas-and-themes.md](new-areas-and-themes.md); where one builds on a planned item, it says so.
What the handheld has today is in [features.md](../firmware/docs/features.md).

The larger ideas say what they are, why they help, how they could work, where the work goes
(**core** is `quantus-core`, **hui** the handheld UI, **desktop** the desktop app), a rough effort
(S: days, M: one or two weeks, L: more) and what is still open. The shorter ones say what the
engine already offers for them. The rule from handheld-menus.md §8 holds: maths, data and state go
into the core, so both shells get them.

State on 2026-11-12. An idea that is taken up moves into plans.md or a plan file of its own and
out of this file.

What the electronics mode shows, and what the editor-style modes below try to repeat: a **visual
editor for one field that compiles to exact equations, solves them, and writes the working up**
(Writeup). Few calculators do this.

## Contents

1. Working faster: [user-defined functions](#1-user-defined-functions),
   [history search and pins](#2-history-search-pins-and-favourites),
   [result references](#3-references-to-earlier-results), [macros](#4-macros-and-small-programs)
2. Understanding: [help key](#5-a-help-key), [practice mode](#7-practice-mode),
   [notes](#8-notes-on-calculations)
3. Everyday maths: [graph](#9-more-on-the-handhelds-graph),
   [uncertainties](#10-measurement-uncertainties), [dates and durations](#11-dates-and-durations),
   [money](#12-percent-and-money), [programmer view](#13-a-programmer-view-for-basis-n)
4. Editor-style modes: [statics](#14-statics-trusses-and-beams),
   [control engineering](#15-control-engineering-block-diagrams), [digital logic](#16-digital-logic-editor),
   [Markov chains](#17-markov-chains-and-graphs), [optics](#18-optics-ray-path)
5. Modes on linear algebra: [dimensional analysis](#19-dimensional-analysis-and-buckingham-π),
   [chemistry](#20-chemistry)
6. More maths: [statistics](#21-statistics-and-regression-the-rest),
   [differential equations](#22-differential-equations), [number theory](#23-number-theory-and-discrete-maths),
   [electronics](#24-electronics-extended), [tuning systems](#25-tuning-systems),
   [small additions](#26-small-additions-to-existing-modes)
7. Getting data in and out: [export](#27-export-a-session), [screenshots](#28-screenshots),
   [USB drive](#29-the-sd-card-as-a-usb-drive), [clipboard](#30-a-shared-clipboard-with-the-desktop),
   [packs on the card](#31-constant-unit-and-formula-packs-on-the-card)
8. The device: [resume where I was](#32-resume-where-i-was), [battery and power](#33-battery-details-and-power-saving),
   [large print](#34-large-print-in-the-menus), [key feedback](#35-key-feedback),
   [exam mode](#36-an-exam-mode-teachers-can-trust), [classroom](#37-worksheets-and-sharing-in-class),
   [calculator link](#38-contacts-sending-and-two-player-games)

[Suggested order](#suggested-order) at the end.

---

## 1. Working faster

### 1. User-defined functions

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

### 2. History search, pins and favourites

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

### 3. References to earlier results

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

### 4. Macros and small programs

**What.** Record a sequence of calculations (with inputs asked for, like CALC) and replay it with
new values. Example: a lab evaluation that computes a mean, a deviation and a result in a unit,
run once per measurement series.

**Why.** Graphing calculators sell on programs; a simple recorded sequence covers most of what
students and technicians use them for, without a programming language.

**How.** A macro is a list of inputs with free variables, saved like a session. Replaying asks
for each free variable once and runs the inputs in order, filling the history. Later, if wanted:
a condition and a loop (a small programming mode on the handheld). Kept in `/macros/` on the card.

**Where.** Core: the macro format and the replay through the API. hui: record, list, run.
Desktop: the same, with an editor.

**Effort.** M (recorded sequences), L (with conditions and loops).

**Open.** Whether macros are allowed in Prüfungsmodus (probably not); how much the Lua apps
already cover.

## 2. Understanding

### 5. A help key

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

### 7. Practice mode

**What.** The user types a task and their own answer; Quantus says right or wrong, and on request
shows the next step of the working as a hint, then the next, before the full solution. Each step
can answer "why?".

**Why.** The step list (Rechenweg) already exists; showing it all at once tempts copying. Step by
step, the calculator becomes a tutor, which suits students at home.

**How.** Builds on Prüfen (equality check) and the step list: the answer is compared with the
result as an equality in the core (so `\frac{1}{2}` and `0.5` both count, with a setting for
whether a decimal counts as exact). Hints are the steps at the chosen detail level, one per key
press.

**Where.** Core: an answer check that accepts equivalent forms. hui and desktop: the mode.

**Effort.** M, after the step list is on the handheld.

**Open.** Whether equivalent but unsimplified answers count as right.

### 8. Notes on calculations

**What.** A short text note on a history entry or on a session ("Aufgabe 3b", "Messreihe 2,
Raum 104").

**Why.** Sessions get dates as names; a note makes a session or a result findable later and
explains it when it is exported.

**How.** A text field in the session JSON per entry and per session. Typed on a USB keyboard or
with ALPHA. Shown in the history in a small font, and in exports.

**Where.** Core: session format. hui and desktop: entry and display.

**Effort.** S.

**Open.** None worth noting.

## 3. Everyday maths

### 9. More on the handheld's graph

**What.** The graph of a curve sketch is there (points marked, pan, zoom, drag). Missing: a trace
cursor that walks along the curve and shows x and f(x), and several functions at once, each in a
tile colour.

**Why.** Reading values off the curve is what a student does next; comparing two functions
(f and g of the Wertetabelle) needs both in one picture.

**How.** The trace takes the core's sample and moves by one pixel's x per key press, asking the
core for the exact f(x) when it stops. Several functions are several samples in one window.

**Where.** Core: a sample for several functions in one window. hui: the cursor, the colours.

**Effort.** S–M.

**Open.** Whether parametric and polar graphs follow.

### 10. Measurement uncertainties

**What.** Values with an uncertainty in Physik: `(9{,}81 \pm 0{,}02)\,\mathrm{m/s^2}`, carried
through the calculation (Gaussian propagation) and rounded in the result as a lab report wants
it: the uncertainty to one or two significant digits, the value to the same place. A breakdown
says where the error comes from ("70 % of the error comes from R2"); Writeup could produce a lab
protocol.

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

### 11. Dates and durations

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

### 12. Percent and money

**What.** Percent changes (`+19 %`, "what % is 30 of 120"), VAT in and out, compound interest,
annuities and an amortisation schedule, IRR, the time value of money (solve for any of n, i, PV,
PMT, FV, like Casio TVM but exact and with steps), and a currency unit with rates fetched over
WLAN on request.

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

### 13. A programmer view for Basis-N

**What.** In Basis-N, a view of the result's bits in groups of four, the word size (8, 16, 32,
64 bit) and signed or unsigned; shifts and rotations; an IEEE-754 view that shows a float's sign,
exponent and mantissa.

**Why.** Basis-N on the Casio is fixed at 32 bit; programmers and electronics students need
other widths and the bit picture, and the electronics mode is in Quantus already.

**How.** Word size and signedness as Basis-N settings in the core; the bit view is a result form
the hui draws. Shifts and rotations as functions.

**Where.** Core: settings, operations, result form. hui: the view. Desktop: the same in its
Basis-N mode.

**Effort.** S–M (Basis-N is on the handheld).

**Open.** Whether the IEEE view belongs here or in a separate tool.

## 4. Editor-style modes (the electronics pattern)

### 14. Statics (trusses and beams)

Draw a beam or truss, place supports, point loads and distributed loads. Results: support
reactions, member forces, shear (Q) and bending moment (M) diagrams, deflection, all as exact
fractions.
- Engine: joint equilibrium is a linear system, like `kirchhoff/` and `linear.rs`. Writeup writes
  it as a worked solution.
- Open: editor design (reuse the circuit editor's canvas?), sign conventions, units.

### 15. Control engineering (block diagrams)

Blocks (P, I, PT1, dead time, summing point, feedback) wired in an editor. Results: overall transfer
function (Mason or block reduction, with steps), poles and zeros, Routh-Hurwitz stability, step
response, Bode plot. Symbolic in parameters: "for which K is this stable?".
- Engine: `algebra/` and `eval/polynomial.rs` for the rational functions; plots exist.
- Open: Laplace-domain input, how much of the symbolic side `algebra/` can carry.

### 16. Digital logic editor

Draw gates; get the truth table, the minimised function (Karnaugh map, Quine-McCluskey), a NAND-only
rewrite and the propagation delay. The digital counterpart of the electronics mode.
- Engine: Base-N and Verify already cover logic operators and relations.
- Spec: [design/digital-logic.md](../design/digital-logic.md).

### 17. Markov chains and graphs

Draw states and arrows with probabilities. Results: stationary distribution, absorption
probabilities, expected number of steps, all as exact fractions. Graph variant: shortest path,
max flow, minimum spanning tree, with steps.
- Engine: exact rationals and `matrix.rs`; exact fractions are the selling point over floating
  point tools.

### 18. Optics ray path

Place lenses, mirrors and distances on an axis. ABCD matrix method gives image distance and
magnification; draw the construction as a plot.
- Engine: `matrix.rs`; plotting from curve sketching.

## 5. Modes built on linear algebra the engine already has

### 19. Dimensional analysis and Buckingham π

Enter quantities (ρ, v, L, μ, …) and get the dimensionless groups (Reynolds number and so on). It
is the null space of the dimension matrix, so a linear system over the unit exponents. Related:
a plausibility check that tells which unit a result must have, and what is missing when an equation
is dimensionally inconsistent.
- Engine: `units/`, `linear.rs`.

### 20. Chemistry

Balance reaction equations automatically (null space of the element matrix), molar masses from
formulas typed as `H_2SO_4`, stoichiometry, pH and buffers, law of mass action.
- Engine: exact linear system, units (mol, g/mol), a small element table in `constants.rs`.

## 6. More maths

### 21. Statistics and regression, the rest

The engine (`stochastics/`) and the handheld's Statistik have the lists, the one- and
two-variable values and the seven regressions. Missing: a statistics view on the desktop, a plot
of the data with the regression curve, and hypothesis tests (t, χ²) with working. See also the
Stochastics item in [new-areas-and-themes.md](new-areas-and-themes.md).

### 22. Differential equations

Separation of variables, first-order linear, second-order with constant coefficients. Optionally a
slope field or a numerical solution (RK4). Physics uses: oscillation, RC discharge, decay.

### 23. Number theory and discrete maths

Modular inverses, Chinese remainder theorem, Euler's φ (`mod`, ggT, kgV and nPr exist). Truth
tables and Boolean simplification fit Base-N and Verify (see also idea 16).

### 24. Electronics, extended

Filters (RC/RLC with Bode plot), voltage dividers, E12/E24 series (nearest standard value), op-amp
circuits, three-phase power, phasor diagram and impedance in Complex.

### 25. Tuning systems

Just, meantone and equal temperament as exact frequency ratios, with cent deviation and the circle
of fifths. Odd but charming: exact arithmetic shows its worth here.

### 26. Small additions to existing modes

- Phasor diagram and Gaussian plane in the Complex mode.
- Taylor series with radius of convergence (`\lim`, `\sum` and `\prod` exist).
- Multivariable calculus: gradient, double integrals, Lagrange multipliers (partial derivatives
  exist).
- CSV export of the value table.
- Parameter study: sliders for `a` in `f(x)=a·x²`, the plot updates live.
- Solver with an interval and all solutions, with a plot of the roots.

## 7. Getting data in and out

### 27. Export a session

**What.** A session written as Markdown, LaTeX (`align*`, as the desktop copies), or PDF: each
input, its result and, if wanted, the steps and notes. The handheld writes it to the SD card.

**Why.** Homework and lab reports need the working on paper or in a document; today the session
JSON is only readable by Quantus.

**How.** The core already writes LaTeX for results and the desktop's writeup (which copies
Markdown). A core function turns a session into Markdown or a LaTeX document; the hui saves it to
`/export/`. PDF on the desktop only (it has a renderer); on the handheld LaTeX and Markdown are
enough.

**Where.** Core: the export. hui: an entry in Sitzungen. Desktop: export in the session menu.

**Effort.** S–M.

**Open.** Whether plain text is wanted as a third format.

### 28. Screenshots

**What.** A key combination saves the screen as a BMP (or PNG) to `/screenshots/` on the card.

**Why.** For questions to a teacher, bug reports, the documentation and the website.

**How.** The renderer draws in bands; a screenshot draws all bands once more into a file instead
of the display. No frame buffer is needed.

**Where.** hui and board.

**Effort.** S.

**Open.** Which keys.

### 29. The SD card as a USB drive

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

### 30. A shared clipboard with the desktop

**What.** "Send to desktop" on a result puts its LaTeX on the desktop's clipboard; the desktop can
send an input to the handheld's editor.

**Why.** The quickest way from a calculation on the device into a document on the PC, and from a
task on the PC onto the device.

**How.** One more message in the planned link between handheld and desktop (plans.md §5), over
WLAN or Bluetooth with the same pairing.

**Where.** hui and desktop, on the link.

**Effort.** S, once the link exists.

**Open.** None beyond the link itself.

### 31. Constant, unit and formula packs on the card

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

**Open.** Packs in Prüfungsmodus (likely off unless the teacher allows them, see idea 36).

## 8. The device

### 32. Resume where I was

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

### 33. Battery details and power saving

**What.** A SETUP page with the charge in percent, the time left at the current use, the cell's
voltage and health, and the charging state; a warning at 10 %. A power-saving mode that dims the
screen and slows down while idle.

**Why.** A battery icon alone does not tell whether the device lasts the exam.

**How.** The MAX17048 gives charge and rate; the estimate uses the average current of the last
minutes. A warning screen once, at 10 %, before the firmware saves and switches off at the
low-voltage threshold.

**Where.** Board and hui.

**Effort.** S, once the gauge is on the PCB.

**Open.** What power saving does beyond Helligkeit and Automatisch aus (both UI only today).

### 34. Large print in the menus

**What.** The large-print setting (SETUP → Gerät → Schriftgröße) for the menus and sheets too, not
only the calculation.

**Why.** For people with low vision a big input next to small menus helps only half way.

**How.** The menus and sheets take their font from the setting and show fewer lines; long entries
scroll or are cut.

**Where.** hui.

**Effort.** M.

**Open.** Whether the soft-key labels grow too.

### 35. Key feedback

**What.** An optional short click from a small buzzer or a vibration motor on each key press,
and a different sound for an error.

**Why.** Low-profile SMD switches under printed caps have little travel; feedback avoids double
presses and missed keys.

**How.** A piezo buzzer or a small ERM motor on a GPIO with a transistor; off by default, a
SETUP entry. Silent in Prüfungsmodus.

**Where.** Hardware (one part on the PCB) and board firmware.

**Effort.** S, but it must go on the PCB before it is made.

**Open.** Buzzer or motor, and the current it draws.

### 36. An exam mode teachers can trust

**What.** Prüfungsmodus that can be seen and checked: a status LED that blinks while it is on, a
log of when it was switched on and off (with the time), and an exam profile chosen by the teacher
that locks modes, packs and macros, and the CAS functions (integrals, solver, symbolic
derivatives) as some schools require. Loaded from a QR code or a file.

**Why.** Today Prüfungsmodus switches the radios off and locks the apps, but a teacher cannot see
that from across a room, and exams differ in what they allow (no CAS, no steps, no stored
formulas). Calculators admitted in German exams need a checkable exam state. Casio-style handhelds
lock CAS only with a hardware switch.

**How.** An LED on the PCB driven only by the firmware in exam mode. The log is kept in NVS and
shown on a page that cannot be cleared while the mode is on. A profile lists the allowed modes
and functions; the hui and the desktop hide the rest and the core refuses them. Leaving the mode
clears the sessions made during it, or keeps them, as the profile says.

**Where.** Hardware (LED), board, hui, desktop, core (refusing functions by profile).

**Effort.** M–L.

**Open.** What the German exam rules (for example the Abitur's) require of a calculator; whether
a profile should need a code to leave.

### 37. Worksheets and sharing in class

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

### 38. Contacts, sending and two-player games

**What.** Two handhelds held together become each other's contacts. From then on either can send
the other a session, as AirDrop does, and the two can play two-player games, four in a row first.

**Why.** Sharing a calculation with the person at the next desk, without a network or a phone.

**How.** NFC (PN7160) for the tap, ESP-NOW for the transport; everything received needs a yes on a
popup. The full design is in [calculator-link.md](../design/calculator-link.md).

**Where.** Hardware (NFC chip and coil), board, hui.

**Effort.** L.

**Open.** See calculator-link.md §13.

## Suggested order

1. **Statics** (14): closest to the electronics mode, widely needed in engineering courses.
2. **Control engineering** (15): a symbolic stability limit is the "wow" case.
3. **Measurement uncertainties** (10): the engine has most of it, clear use in labs.
4. Then dimensional analysis, chemistry, money, the rest of statistics.
