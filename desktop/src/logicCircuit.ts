import type { GateType, LogicScene } from './api';
import type { TextKey } from './i18n';
import { freshIdOf, GRID, rotate, type Placeable, type Point } from './schematic';

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
}

/** The editor's parts kept in a scene, none if it has none. */
export function editorParts(scene: LogicScene): LogicPart[] {
  const data = scene.editor as Partial<LogicEditorData> | null | undefined;
  return Array.isArray(data?.parts) ? data.parts : [];
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
