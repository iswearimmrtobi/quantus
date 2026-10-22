# Test inputs: what the engine supports and what it doesn't (yet)

Every input below was run through `LatexCalculator::evaluate` (precision 50, no session
variables) on 2026-10-22. Error messages are quoted exactly as the engine returns them.
The parser stops at the first problem, so an input with several unsupported commands only
shows the first one.

**Result: 28 of 48 inputs work and give the expected value; 20 are unsupported.** No input
produced a wrong number.

## Unsupported

| Feature | Error | Inputs |
|---|---|---|
| `\sin`, `\cos` | `Unsupported command '\sin'.` / `'\cos'` | P2, P3, P5, U3, U5 (and M11, hidden behind `\lim`) |
| `\int` (single and nested) | `Unsupported command '\int'.` | M9, P6, P7, P8, U7, U8 |
| `\log`, `\ln` | `Unsupported command '\log'.` | M3 (`\ln` is hidden behind `\log`) |
| `\lim` | `Unsupported command '\lim'.` | M11 |
| `\sum` | `Unsupported command '\sum'.` | M6 |
| `\binom` | `Unsupported command '\binom'.` | M7 |
| `\det`, `pmatrix` | `Unsupported command '\det'.` | M12 |
| `\|…\|` (absolute value) | `Unexpected character '\|'.` | M8 |
| Variables without an index | `'x' is neither a unit nor a constant. Variables must carry a subscript, e.g. v_1.` | M4, M5 |
| Derivative `\frac{d}{dx}` | `'dx' is neither a unit nor a constant. …` (read as a unit run) | M10 |
| Equations and solving | not reachable: `x^2 - 5x + 6 = 0` fails on `x` first, and `=` is only a label | M4, M5 |

Notes:

- **Trig** is the largest gap for the physics inputs. Five of them (projectile range and
  height, resultant force) only fail because of `\sin`/`\cos`. With `\pi` already available,
  trig needs radians first and degrees later (`30^\circ`).
- **Integrals** with units (U7, U8) also need the integration variable to carry a unit, for
  example `x` in m.
- **Plain-dot decimals work.** `9.81` is accepted next to `9{,}81`, so none of the failures
  above come from the decimal separator.

## Supported, with the expected result

| ID | Input (short) | Result |
|---|---|---|
| M1 | `\frac{(3+4)^2 - 5\cdot 2}{\sqrt{16}+3}` | 5.5714… (39/7) |
| M2 | nested `\frac` chain | 0.6 |
| P1 | `\frac{20^2}{2\cdot 0.5\cdot 9.81}` | 40.7747… |
| P4 | `2\pi\sqrt{\frac{2.5}{9.81}}` | 3.17187… |
| P9 | parallel resistors, power | 2.53091… |
| P10a, P10b | ΔT and heat, no units | 180, 10475.64 |
| P11 | mixing temperature, no units | 35 |
| P12 | thin lens | 0.16667 |
| P13 | Earth–Moon force, scientific notation | 1.98202… · 10²⁰ |
| P14 | orbit speed | 7672.59… |
| P15 | Lorentz factor | 1.6667 |
| U1 | stopping distance | 40.7747… m |
| U2 | kinetic energy | 300000 J (named *energy*) |
| U4 | pendulum period | 3.17187… s |
| U6 | power | 1000 W (named *power*) |
| U9 | parallel resistors, power | 2.53091… W |
| U10a | ΔT | 180 (input had no unit) |
| U10b | heat, isobaric | 10475.64 J |
| U11 | mixing temperature | 308.15 K |
| U12 | thin lens | 0.16667 m |
| U13 | Earth–Moon force with units | 1.98202… · 10²⁰ N (named *force*) |
| U14 | orbit speed with units | 7672.59… m/s |
| U15 | density | 2700 kg/m³ |
| U16 | `1 km + 500 m` | 1.5 km |
| U17 | `36 km/h · 10 s` | 100 m |
| U18 | `5 m + 3 s` | error: `Incompatible units: m (length) and s (time) cannot be added.` (correct) |
| U19 | `\sqrt{9\,\mathrm{m}}` | 3 m^(1/2) (accepted as the fractional power) |

U10a and P10a are the same input without a unit. The unit version in the test list asked for
`180 K` from a dimensionless expression, so only the number is checked.

## Input IDs

`M` is the general maths list, `P` the physics list without units, `U` the physics list with
units. The full inputs are in the conversation; the table above shows the failing commands.
