---
sidebar_position: 2
title: Units and constants
---

# Units and constants

Units are known in Physics mode. In the other modes write them in
`\mathrm{…}`. A unit symbol is looked up as a whole, so `min` is the minute (not $m \cdot i \cdot n$),
`mN` the millinewton and `Pa` the pascal.

## SI units

These take every metric prefix (`km`, `mA`, `µF`, `GHz`, …).

| Quantity | Unit | Symbol |
|---|---|---|
| length | metre | `m` |
| mass | gram, kilogram | `g`, `kg` |
| time | second | `s` |
| electric current | ampere | `A` |
| temperature | kelvin, degree Celsius | `K`, `°C` |
| amount of substance | mole | `mol` |
| luminous intensity | candela | `cd` |
| angle, solid angle | radian, steradian | `rad`, `sr` |
| frequency | hertz | `Hz` |
| force | newton | `N` |
| pressure | pascal | `Pa` |
| energy | joule | `J` |
| power | watt | `W` |
| electric charge | coulomb | `C` |
| voltage | volt | `V` |
| capacitance | farad | `F` |
| resistance | ohm | `\Omega`, `Ohm` |
| conductance | siemens | `S` |
| magnetic flux | weber | `Wb` |
| magnetic flux density | tesla | `T` |
| inductance | henry | `H` |
| luminous flux, illuminance | lumen, lux | `lm`, `lx` |
| activity | becquerel | `Bq` |
| absorbed dose, dose equivalent | gray, sievert | `Gy`, `Sv` |
| catalytic activity | katal | `kat` |

Prefixes: `q r y z a f p n µ m c d da h k M G T P E Z Y R Q`. Micro can be typed as `µ`, `μ`
or `\mu` (`3\mu F`); `u` is not micro but the dalton.

## Other units

| Unit | Symbol | Prefixes |
|---|---|---|
| litre | `l` | yes |
| minute, hour, day, week | `min`, `h`, `d` (`day`), `wk` (`week`) | |
| month, year | `mo` (`mon`, `month`), `yr` (`year`; `a` and `y` see below) | |
| electronvolt | `eV` | yes |
| bar, atmosphere, torr | `bar`, `atm`, `Torr` | `bar` only |
| calorie, watt-hour | `cal`, `Wh` | yes (`kcal`, `kWh`) |
| hectare | `ha` | |
| ångström | `Å` (`\AA`) | |
| mile per hour | `mph` | |
| light-year, astronomical unit | `ly`, `AU` | |
| dalton | `Da` (`u`, see below) | |
| percent | `\%` | |
| tonne, inch | `t`, `in` (see below) | |

Imperial and US units, and the rest of the conversions of a Casio fx-991DE X (NIST SP 811):

| Quantity | Unit | Symbol |
|---|---|---|
| length | foot, yard, mile, nautical mile, mil, fathom, chain, parsec | `ft`, `yd`, `mi`, `nmi`, `mil`, `fath`, `ch`, `pc` |
| area | acre | `acre` |
| volume | US gallon, US bushel, oil barrel | `gal`, `bu`, `bbl` |
| mass | pound, ounce, carat, short ton | `lb`, `oz`, `ct`, `ton` |
| speed, acceleration | knot, gal | `kn`, `Gal` |
| force | dyne, pound-force, kilogram-force | `dyn`, `lbf`, `kgf` |
| pressure | pound per square inch, millimetre and inch of mercury | `psi`, `mmHg`, `inHg` |
| energy, power | erg, British thermal unit, horsepower | `erg`, `Btu`, `hp` |
| temperature | degree Fahrenheit | `°F`, `^\circ\mathrm{F}` |
| kinematic viscosity | stokes | `St` |
| magnetic flux, activity, dose | maxwell, curie, rem | `Mx`, `Ci`, `rem` |

Left out because the letter is a common variable or the symbol is taken: barn (`b`), poise
(`P`), gauss (`G`), röntgen (`R`), are (`a`), rad as a dose (`rad` is the radian), the Imperial
gallon, the long ton, and the oersted and the IT and 15 °C calories.

Some letters are variables by default because they are more often meant that way. Two settings
under **Settings → Input** turn them into units:

| Letters | Unit | Setting |
|---|---|---|
| `a`, `y` | year | **a and y as variables** off |
| `t`, `u`, `in` | tonne, dalton, inch | **t, u and in as units** on |

## Named results

A result whose unit came out of the arithmetic is written with the named unit of its dimension,
if there is one: $\mathrm{kg \cdot m/s^2}$ becomes $\mathrm{N}$. Units you typed yourself are
kept (`72\frac{km}{h}` stays in $\mathrm{km/h}$). Where two units share a dimension, the first
wins: hertz over becquerel, gray over sievert, kelvin over degree Celsius.

The result also carries the name of the quantity (*force*, *velocity*, *density*, …), which the
error messages use too: *m (length) and s (time) cannot be added.*

## Physical constants

Physics mode only (and the circuit modes), CODATA 2018: the 47 constants of a Casio fx-991DE X
in its six categories. A session variable with the same name wins. Each is typed by its key,
with the subscript in braces or without (`\mu_{N}` or `\mu_N`).

| Category | Constant | Input |
|---|---|---|
| Universal | Planck constant, reduced ($h_P/2\pi$) | `h_P`, `\hbar` |
| | speed of light in vacuum | `c_0` |
| | vacuum permittivity, permeability | `\varepsilon_0` (`\epsilon_0`), `\mu_0` |
| | characteristic impedance of vacuum | `Z_0` |
| | gravitational constant | `G_N` |
| | Planck length, Planck time | `l_P`, `t_P` |
| Electromagnetic | nuclear magneton, Bohr magneton | `\mu_N`, `\mu_B` |
| | elementary charge | `e_0` |
| | magnetic flux quantum, conductance quantum | `\Phi_0`, `G_0` |
| | Josephson constant, von Klitzing constant | `K_J`, `R_K` |
| Atomic and nuclear | proton, neutron, electron, muon, tau mass | `m_p`, `m_n`, `m_e`, `m_\mu`, `m_\tau` |
| | Bohr radius, classical electron radius | `a_0`, `r_e` |
| | fine-structure constant | `\alpha_f` |
| | Compton wavelength (electron, proton, neutron) | `\lambda_C`, `\lambda_{Cp}`, `\lambda_{Cn}` |
| | proton gyromagnetic ratio | `\gamma_p` |
| | Rydberg constant | `R_{inf}` |
| | magnetic moment of proton, electron, neutron, muon | `\mu_p`, `\mu_e`, `\mu_n`, `\mu_\mu` |
| Physico-chemical | atomic mass constant | `m_u` |
| | Faraday constant | `F_{Fa}` |
| | Avogadro, Boltzmann, molar gas constant | `N_A`, `k_B`, `R_m` |
| | molar volume of an ideal gas (273,15 K, 101,325 kPa) | `V_m` |
| | first and second radiation constant | `c_1`, `c_2` |
| | Stefan–Boltzmann constant | `\sigma_{SB}` |
| Adopted values | standard gravity, standard atmosphere | `g_n`, `p_{atm}` |
| | conventional von Klitzing and Josephson constant (1990) | `R_{K90}`, `K_{J90}` |
| Other | 0 °C in kelvin | `T_0` |

The engine lists them with their values for a picker (`constants`, see the
[engine API](../contributing/api.md#settings-and-info)).
