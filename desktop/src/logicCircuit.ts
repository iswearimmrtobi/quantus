import type { DrawForm, GateType, LogicElement, LogicOutput, LogicScene, LogicTaskResult, Netlist } from './api';
import type { TextKey } from './i18n';
import { freshIdOf, GRID, inside, pointKey, rotate, wireEnds, type Placeable, type Point } from './schematic';

/**
 * The Logic mode's circuit as the editor keeps it (design/digital-logic.md §5.5): switches, lamps,
 * constants and gates on the grid, joined by wires. It rides along in `LogicScene.editor`; the
 * core gets the netlist made from it.
 */

export type PartKind = 'input' | 'output' | 'constant' | 'gate' | 'wire';

export interface LogicPart extends Placeable {
  kind: PartKind;
  /** A switch's variable or a lamp's output. */
  name?: string;
  /** A constant's value. */
  value?: boolean;
  gate?: GateType;
  /** A gate's number of inputs, 2 to 8; NOT and buffer have one. */
  inputs?: number;
}

/** What the scene keeps for the editor. */
export interface LogicEditorData {
  parts: LogicPart[];
  /**
   * How the circuit was drawn, while it has not been edited since (§5.1): the form, the output
   * (all when null), and what it was drawn from, so that it is drawn again when that changes.
   */
  generated?: { form: DrawForm; output: string | null; from: string } | null;
}

/** What the scene keeps for the editor, none if it has nothing. */
export function editorData(scene: LogicScene): LogicEditorData {
  const data = scene.editor as Partial<LogicEditorData> | null | undefined;
  return { parts: Array.isArray(data?.parts) ? data.parts : [], generated: data?.generated ?? null };
}

/** The editor's parts kept in a scene, none if it has none. */
export function editorParts(scene: LogicScene): LogicPart[] {
  return editorData(scene).parts;
}

/** What the palette offers: the switch, the lamp, a constant, each gate type and the wire. */
export type PaletteKind = 'input' | 'output' | 'constant' | GateType | 'wire';

export const LOGIC_PALETTE: PaletteKind[] = ['input', 'output', 'constant', 'not', 'buffer', 'and', 'or', 'nand', 'nor', 'xor', 'xnor', 'wire'];

export const GATES: GateType[] = ['and', 'or', 'nand', 'nor', 'xor', 'xnor', 'not', 'buffer'];

/** The most inputs of a gate, as of a scene. */
export const MAX_GATE_INPUTS = 8;

/** Whether a gate has one input. */
export const single = (gate: GateType | undefined) => gate === 'not' || gate === 'buffer';

/** Whether a gate's output is inverted, drawn with a bubble. */
export const inverted = (gate: GateType | undefined) => gate === 'not' || gate === 'nand' || gate === 'nor' || gate === 'xnor';

/** A gate's name as in textbooks of either language. */
export const gateName = (gate: GateType) => (gate === 'buffer' ? 'Buffer' : gate.toUpperCase());

export const PART_LABELS: Record<Exclude<PartKind, 'gate'>, TextKey> = {
  input: 'logic.part.input',
  output: 'logic.part.output',
  constant: 'logic.part.constant',
  wire: 'kind.wire',
};

/** The number of inputs of a part: a gate's own, one for a lamp, none else. */
export function inputCount(part: LogicPart): number {
  if (part.kind === 'gate') return single(part.gate) ? 1 : Math.min(MAX_GATE_INPUTS, Math.max(2, part.inputs ?? 2));
  return part.kind === 'output' ? 1 : 0;
}

/**
 * The rows of a gate's input pins relative to its output, in grid steps: one apart, centred, the
 * output's row left out for an even number; as the core lays them out (`layout::pin_offsets`).
 */
export function pinOffsets(inputs: number): number[] {
  if (inputs % 2 === 1) return Array.from({ length: inputs }, (_, i) => i - (inputs - 1) / 2);
  const half = inputs / 2;
  return [...Array.from({ length: half }, (_, i) => -(half - i)), ...Array.from({ length: half }, (_, i) => i + 1)];
}

/**
 * The pins of a part unturned, in drawing units: a gate's inputs two grid steps left of its place in
 * the rows of [`pinOffsets`], its output two steps right; a switch's or a constant's output one
 * step right, a lamp's input one step left; as the core's layout places them.
 */
function localPins(part: LogicPart): { inputs: Point[]; output: Point | null } {
  switch (part.kind) {
    case 'gate':
      return { inputs: pinOffsets(inputCount(part)).map((o) => ({ x: -2 * GRID, y: o * GRID })), output: { x: 2 * GRID, y: 0 } };
    case 'input':
    case 'constant':
      return { inputs: [], output: { x: GRID, y: 0 } };
    case 'output':
      return { inputs: [{ x: -GRID, y: 0 }], output: null };
    case 'wire':
      return { inputs: [], output: null };
  }
}

/** The pins of a part at their places on the canvas: the inputs in order and the output. */
export function partPins(part: LogicPart): { inputs: Point[]; output: Point | null } {
  const place = (p: Point) => {
    const turned = rotate(p, part.rotation);
    return { x: part.x + turned.x, y: part.y + turned.y };
  };
  const { inputs, output } = localPins(part);
  return { inputs: inputs.map(place), output: output && place(output) };
}

/** Every pin of a part, both ends of a wire. */
export function allPins(part: LogicPart): Point[] {
  if (part.kind === 'wire') return [{ x: part.x, y: part.y }, { x: part.x2 ?? part.x, y: part.y2 ?? part.y }];
  const { inputs, output } = partPins(part);
  return output ? [...inputs, output] : inputs;
}

/** Half the height of a gate's body: half a grid step beyond its outermost pins. */
export function bodyHalf(part: LogicPart): number {
  const offsets = pinOffsets(inputCount(part));
  return single(part.gate) ? 0.8 * GRID : (Math.max(...offsets.map(Math.abs)) + 0.5) * GRID;
}

/** The drawn body of a part unturned, from its place: left, top, right, bottom. */
export function localBody(part: LogicPart): [number, number, number, number] {
  switch (part.kind) {
    case 'gate': {
      const h = bodyHalf(part);
      return [-GRID, -h, inverted(part.gate) ? GRID + 8 : GRID, h];
    }
    case 'input':
      return [-16, -9, 8, 9];
    case 'constant':
      return [-12, -9, 8, 9];
    case 'output':
      return [-9, -9, 9, 9];
    case 'wire':
      return [0, 0, 0, 0];
  }
}

const PREFIX: Record<PartKind, string> = { input: 'I', output: 'O', constant: 'K', gate: 'G', wire: 'W' };

const LETTERS = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ'.split('');
const OUTPUT_NAMES = 'fghpqrsuvwyz'.split('');

/** The first of `candidates` that no part of `kind` is named, then `base_1`, `base_2`, … */
function freeName(parts: LogicPart[], kind: PartKind, candidates: string[], base: string): string {
  const taken = new Set(parts.filter((p) => p.kind === kind).map((p) => p.name));
  return candidates.find((name) => !taken.has(name)) ?? freshIdOf(taken as Set<string>, `${base}_`);
}

/** A new part of a palette kind at the grid point `at`. */
export function newPart(parts: LogicPart[], kind: PaletteKind, at: Point): LogicPart {
  const partKind: PartKind = kind === 'input' || kind === 'output' || kind === 'constant' || kind === 'wire' ? kind : 'gate';
  const base: LogicPart = { id: freshIdOf(parts.map((p) => p.id), PREFIX[partKind]), kind: partKind, x: at.x, y: at.y, rotation: 0 };
  switch (partKind) {
    case 'input':
      return { ...base, name: freeName(parts, 'input', LETTERS, 'x') };
    case 'output':
      return { ...base, name: freeName(parts, 'output', OUTPUT_NAMES, 'f') };
    case 'constant':
      return { ...base, value: true };
    case 'wire':
      return { ...base, x2: at.x + 4 * GRID, y2: at.y };
    case 'gate': {
      const gate = kind as GateType;
      return { ...base, gate, inputs: single(gate) ? 1 : 2 };
    }
  }
}

/** A gate changed to `gate`: one input for NOT and buffer, two when it had one. */
export function withGate(part: LogicPart, gate: GateType): LogicPart {
  return { ...part, gate, inputs: single(gate) ? 1 : Math.max(2, part.inputs ?? 2) };
}

// ---- The netlist (§5.5): nets from what touches.

/** The circuit's nets: each pin and wire end on a net, numbered from 1. */
export interface Nets {
  /** The net at a grid point (`pointKey`) where a pin or wire end sits. */
  byPoint: Map<string, number>;
  /** How many pins and wire ends each net joins; a pin alone is not connected. */
  size: Map<number, number>;
}

/**
 * Numbers the nets: pins and wire ends on the same point are joined, and every wire joins its two
 * ends and whatever sits in its middle (the Logic mode's wires have no resistance).
 */
export function numberNets(parts: LogicPart[]): Nets {
  const parent = new Map<string, string>();
  const root = (key: string): string => {
    let k = key;
    while (parent.has(k)) k = parent.get(k)!;
    return k;
  };
  const join = (p: string, q: string) => {
    const [a, b] = [root(p), root(q)];
    if (a !== b) parent.set(a, b);
  };
  const points = new Map<string, Point>();
  const count = new Map<string, number>();
  for (const part of parts) {
    for (const point of allPins(part)) {
      const key = pointKey(point);
      points.set(key, point);
      count.set(key, (count.get(key) ?? 0) + 1);
    }
  }
  for (const wire of parts.filter((p) => p.kind === 'wire')) {
    const [a, b] = wireEnds(wire);
    join(pointKey(a), pointKey(b));
    for (const [key, point] of points) if (inside(a, b, point)) join(key, pointKey(a));
  }
  const number = new Map<string, number>();
  const byPoint = new Map<string, number>();
  const size = new Map<number, number>();
  for (const key of points.keys()) {
    const r = root(key);
    if (!number.has(r)) number.set(r, number.size + 1);
    const net = number.get(r)!;
    byPoint.set(key, net);
    size.set(net, (size.get(net) ?? 0) + count.get(key)!);
  }
  return { byPoint, size };
}

/**
 * The netlist the core checks: every part but the wires, with the nets of its pins; a pin that
 * touches nothing is not connected. Null without any part.
 */
export function toNetlist(parts: LogicPart[], nets: Nets = numberNets(parts)): Netlist | null {
  const net = (point: Point | null) => {
    if (!point) return null;
    const n = nets.byPoint.get(pointKey(point));
    return n !== undefined && (nets.size.get(n) ?? 0) > 1 ? n : null;
  };
  const elements: LogicElement[] = [];
  for (const part of parts) {
    const { inputs, output } = partPins(part);
    switch (part.kind) {
      case 'input':
        elements.push({ id: part.id, kind: 'input', name: part.name ?? '', output: net(output) });
        break;
      case 'output':
        elements.push({ id: part.id, kind: 'output', name: part.name ?? '', inputs: [net(inputs[0])] });
        break;
      case 'constant':
        elements.push({ id: part.id, kind: 'constant', value: part.value ?? false, output: net(output) });
        break;
      case 'gate':
        elements.push({ id: part.id, kind: 'gate', gate: part.gate ?? 'and', inputs: inputs.map(net), output: net(output) });
        break;
    }
  }
  return elements.length ? { elements } : null;
}

/**
 * The scene with the circuit drawn as `parts` (§5.1). When its netlist changes, every output a lamp
 * names comes from the circuit; an output from the circuit follows its lamp when renamed, and goes
 * with it. `names` holds each output's name as the core read it (a line's own `f = …`).
 */
export function withParts(scene: LogicScene, parts: LogicPart[], names: (string | null | undefined)[] = []): LogicScene {
  const before = editorParts(scene);
  const circuit = toNetlist(parts);
  const next: LogicScene = { ...scene, editor: { parts } satisfies LogicEditorData, circuit };
  if (JSON.stringify(circuit) === JSON.stringify(scene.circuit ?? null)) return next;
  const lamps = (list: LogicPart[]) => new Map(list.filter((p) => p.kind === 'output').map((p) => [p.id, p.name ?? '']));
  const [old, now] = [lamps(before), lamps(parts)];
  const renamed = new Map<string, string | null>();
  for (const [id, name] of old) if (now.get(id) !== name) renamed.set(name, now.get(id) ?? null);
  const lampNames = new Set(now.values());
  const outputs = scene.outputs.flatMap((output, index): LogicOutput[] => {
    const name = names[index] ?? output.name;
    if (output.source.kind === 'circuit' && name !== undefined && renamed.has(name)) {
      const to = renamed.get(name)!;
      return to === null ? [] : [{ ...output, name: to }];
    }
    return name !== undefined && lampNames.has(name) ? [{ ...output, name, source: { kind: 'circuit' } }] : [output];
  });
  return { ...next, outputs: outputs.length ? outputs : [{ source: { kind: 'expression', latex: '' } }] };
}

// ---- Circuits from the core (§6.6): drawn from a form, or arranged.

/** A circuit the core drew or arranged: its netlist on the grid. */
export type Generated = NonNullable<LogicTaskResult['circuit']>;

/**
 * The parts of a circuit from the core, unturned at their places on the grid, with wires between
 * the pins; the core's ids. `kept` are parts of the drawing before that the core left out (a
 * constant that drives nothing), which stay where they were.
 */
export function fromGenerated(circuit: Generated, kept: LogicPart[] = []): LogicPart[] {
  const at = new Map(circuit.layout.elements.map((p) => [p.id, p.at]));
  const parts: LogicPart[] = [];
  for (const element of circuit.netlist.elements) {
    const place = at.get(element.id);
    if (!place) continue;
    const base = { id: element.id, x: place.x * GRID, y: place.y * GRID, rotation: 0 };
    switch (element.kind) {
      case 'input':
      case 'output':
        parts.push({ ...base, kind: element.kind, name: element.name });
        break;
      case 'constant':
        parts.push({ ...base, kind: 'constant', value: element.value });
        break;
      case 'gate':
        parts.push({ ...base, kind: 'gate', gate: element.gate, inputs: element.inputs?.length ?? 2 });
        break;
    }
  }
  const ids = new Set(parts.map((p) => p.id));
  parts.push(...kept.filter((p) => p.kind !== 'wire' && !ids.has(p.id)));
  circuit.layout.wires.forEach((wire, k) => {
    parts.push({ id: `W${k + 1}`, kind: 'wire', x: wire.from.x * GRID, y: wire.from.y * GRID, x2: wire.to.x * GRID, y2: wire.to.y * GRID, rotation: 0 });
  });
  return parts;
}

/** What a generated circuit is drawn from: the outputs' sources and choices, the inputs and the 2-input setting. */
export const drawnFrom = (scene: LogicScene, twoInput: boolean) => JSON.stringify([scene.inputs, scene.outputs, twoInput]);

/**
 * The scene with a circuit the core drew from `form` of the scene `from` ([`drawnFrom`]): the
 * outputs keep their sources, since the circuit follows them until it is edited.
 */
export function withGenerated(scene: LogicScene, circuit: Generated, form: DrawForm, output: string | null, from: string): LogicScene {
  const parts = fromGenerated(circuit);
  const editor: LogicEditorData = { parts, generated: { form, output, from } };
  return { ...scene, editor, circuit: toNetlist(parts) };
}
