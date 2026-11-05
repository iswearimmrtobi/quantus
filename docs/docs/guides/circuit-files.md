---
sidebar_position: 7.5
title: Circuit files
---

# Circuit files

**Electronics** and **Kirchhoff** keep their circuit in a `.json` file. **Save** under *File* in
the parts column writes the circuit as drawn, **Load** reads one back and replaces the circuit on
the canvas. Both modes read and write the same format, so a circuit drawn in one opens in the
other. The [example files](#example-files) below are such files.

A file that can't be read is reported, and the circuit on the canvas stays as it was.

## Structure

A 12 V source with a 1 kΩ resistor, written by hand:

```json
{
  "version": 1,
  "elements": [
    { "id": "V1", "kind": "voltageSource", "u": "12V", "x": 100, "y": 200, "rotation": 1 },
    { "id": "R1", "kind": "resistor", "r": "1k\\Omega", "x": 260, "y": 200, "rotation": 1 },
    { "id": "W1", "kind": "wire", "x": 100, "y": 160, "x2": 100, "y2": 120 },
    { "id": "W2", "kind": "wire", "x": 100, "y": 120, "x2": 260, "y2": 120 },
    { "id": "W3", "kind": "wire", "x": 260, "y": 120, "x2": 260, "y2": 160 },
    { "id": "W4", "kind": "wire", "x": 100, "y": 240, "x2": 260, "y2": 240 }
  ]
}
```

| Key | Content |
|---|---|
| `version` | always `1` |
| `elements` | the parts, one object each, in any order |
| `nodes` | written by Save: the node numbers of the drawing |

## A part

| Key | Content |
|---|---|
| `id` | its name on the canvas, unique: `R1`, `V2`, `W3`, `GND1`; the terminals are `A` and `B` |
| `kind` | `resistor`, `wire`, `voltageSource`, `currentSource`, `potentiometer`, `powerLoad`, `ground` or `terminal` (Terminal A/B) |
| `x`, `y` | its middle on the canvas, in pixels; a wire's start |
| `x2`, `y2` | a wire's end |
| `rotation` | quarter turns clockwise, `0` to `3`, `0` if left out; a resistor at 45° has a half, such as `0.5` |
| `reversed` | Kirchhoff only: `true` if you reversed the current arrow of the part's branch |
| `a`, `b`, `wiper` | written by Save: the nodes its terminals are on |
| the values | as in the fields, see below |

The grid has a step of 20 pixels. A part is 80 pixels long, with its terminals `a` and `b` two
steps left and right of its middle at `rotation` 0; at `rotation` 1, `a` is at the top. A source's
`a` is its `+` end. A potentiometer's `wiper` is two steps above its middle. A wire runs across or
down, not on a slant.

Two terminals on the same grid point are connected, and so is a terminal on a wire. Load works the
nodes out from the positions again, so `nodes`, `a`, `b` and `wiper` can be left out of a file
written by hand; Save writes them, so that the file also has the form the solver takes.

## Values

Each value is a string of LaTeX, as you would type it into the field, with its unit. A value left
out or empty is missing, as an empty field is.

| `kind` | Keys | Example |
|---|---|---|
| `resistor` | `r` | `"10\\Omega"` |
| `voltageSource` | `u`, `internalResistance` | `"230V"`, `"0{,}5\\Omega"` |
| `currentSource` | `i`, `internalConductance` | `"2A"`, `"1mS"` |
| `potentiometer` | `rTotal`, `position` (0 to 1) | `"10k\\Omega"`, `"0{,}25"` |
| `powerLoad` | `p` | `"500W"` |
| `wire` | `material`, `length`, `area`, `temp`, `rOverride` | `"copper"`, `"100m"`, `"2mm^2"`, `"20°C"`, `"0{,}1\\Omega"` |
| `ground`, `terminal` | none | |

`material` is one of `copper`, `aluminium`, `silver`, `gold`, `iron`, `constantan` or `brass`.
A wire without a material and without `rOverride` is ideal. In JSON a backslash is written twice:
`"1k\\Omega"`.

## What is refused

Load and Save check the shape of the file. The values themselves are checked when the circuit is
solved, as if you had typed them.

| Problem | Message |
|---|---|
| No `version` | This is not a circuit file: the version is missing. |
| Another `version` | The circuit file has the unknown version 2. |
| No `elements` | The circuit file has no list of parts. |
| A part without `id` | Part 3: the ID is missing. |
| Two parts with the same `id` | The ID R1 occurs more than once. |
| An unknown `kind` | R1: unknown kind of part “capacitor”. |
| No `x` or `y` | R1: the position (x) is missing. |
| `reversed` not `true` or `false` | R1: “reversed” must be true or false. |
| Not JSON | Not a valid circuit file: … |

A file from Electronics can hold what Kirchhoff can't use: a potentiometer, a power load or a
wire with a material is refused there with a message, and ground symbols are ignored.

## Example files

The repository's [`test/circuits/`](https://github.com/iswearimmrtobi/quantus/tree/dev/test/circuits/) holds circuits drawn in the
editor, with their expected values in its [README](https://github.com/iswearimmrtobi/quantus/blob/dev/test/circuits/README.md). Download one and open it with
**Load**. For the resistance between two points in Kirchhoff:

| File | Network | $R_{AB}$ |
|---|---|---|
| [`kirchhoff-series-parallel.json`](https://github.com/iswearimmrtobi/quantus/blob/dev/test/circuits/kirchhoff-series-parallel.json) | eight resistors, series and parallel only | $\frac{2430}{799}\,\Omega$ |
| [`kirchhoff-bridge.json`](https://github.com/iswearimmrtobi/quantus/blob/dev/test/circuits/kirchhoff-bridge.json) | a bridge of five resistors: node rule or one star–delta conversion | $\frac{61}{21}\,\Omega$ |
| [`kirchhoff-two-conversions.json`](https://github.com/iswearimmrtobi/quantus/blob/dev/test/circuits/kirchhoff-two-conversions.json) | seven resistors of 1 Ω, more than one star–delta conversion | $\frac{8}{7}\,\Omega$ |

The others are for Electronics (`divider.json`, `wheatstone.json`, `cable.json`, …) and for the
Kirchhoff working (`kirchhoff-current-source.json`, `kirchhoff-crossing.json`,
`kirchhoff-source-loop.json`).

## See also

- [Electronics](electronics.md)
- [Kirchhoff](kirchhoff/index.md)
- [Resistance between two points](kirchhoff/resistance.md): the terminals A and B
