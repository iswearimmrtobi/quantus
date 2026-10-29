---
sidebar_position: 2
title: Units and constants
---

# Units and constants

Units are known in Physics and Electronics mode. In the other modes write them in
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

Physics mode only (CODATA 2018). A session variable with the same name wins.

| Constant | Input | Value |
|---|---|---|
| speed of light | `c_0` | $299792458\,\mathrm{m/s}$ |
| Planck constant | `h_P` | $6{,}62607015 \cdot 10^{-34}\,\mathrm{J \cdot s}$ |
| reduced Planck constant | `\hbar` | $h_P / 2\pi$ |
| Boltzmann constant | `k_B` | $1{,}380649 \cdot 10^{-23}\,\mathrm{J/K}$ |
| Avogadro constant | `N_A` | $6{,}02214076 \cdot 10^{23}\,\mathrm{mol^{-1}}$ |
| molar gas constant | `R_m` | $8{,}31446261815324\,\mathrm{J/(mol \cdot K)}$ |
| elementary charge | `e_0` | $1{,}602176634 \cdot 10^{-19}\,\mathrm{C}$ |
| vacuum permittivity | `\varepsilon_0`, `\epsilon_0` | $8{,}8541878128 \cdot 10^{-12}\,\mathrm{F/m}$ |
| vacuum permeability | `\mu_0` | $1{,}25663706212 \cdot 10^{-6}\,\mathrm{N/A^2}$ |
| electron, proton, neutron mass | `m_e`, `m_p`, `m_n` | $9{,}109\ldots \cdot 10^{-31}\,\mathrm{kg}$, … |
| Stefan–Boltzmann constant | `\sigma_SB` | $5{,}670374419 \cdot 10^{-8}\,\mathrm{W/(m^2 \cdot K^4)}$ |
| standard gravity | `g_n` | $9{,}80665\,\mathrm{m/s^2}$ |
| gravitational constant | `G_N` | $6{,}67430 \cdot 10^{-11}\,\mathrm{m^3/(kg \cdot s^2)}$ |
