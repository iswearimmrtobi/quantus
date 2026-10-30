import type { TextKey } from './i18n/en';
import type { CircuitElement, CircuitRequest, ElementKind, KirchhoffElement, KirchhoffRequest } from './api';

/**
 * The schematic as the editor keeps it, and the pure functions that turn it into a
 * `CircuitRequest`. Nodes are never typed in: two terminals are connected when they sit on the
 * same grid point, or one sits in the middle of an ideal wire, so the numbers the solver needs
 * come out of the geometry.
 */

export const GRID = 20;

/** `ground` is the reference potential (node 0); the solver never sees it as an element. */
export type EditorKind = ElementKind | 'ground';

/**
 * `a`, `b` and `wiper` are filled in by [`toRequest`]; what the editor stores is the position.
 * A wire runs from (x, y) to (x2, y2); every other part sits at (x, y), turned by
 * `rotation` quarter turns.
 */
export interface EditorElement extends Omit<CircuitElement, 'kind' | 'a' | 'b'> {
  kind: EditorKind;
  x: number;
  y: number;
  rotation: number;
  x2?: number;
  y2?: number;
  /** Kirchhoff mode: the branch this part leads runs the other way. */
  reversed?: boolean;
}

export interface Point {
  x: number;
  y: number;
}

export type Terminal = 'a' | 'b' | 'wiper' | 'ground';

const PREFIX: Record<EditorKind, string> = {
  resistor: 'R',
  wire: 'W',
  voltageSource: 'V',
  currentSource: 'I',
  potentiometer: 'P',
  powerLoad: 'L',
  ground: 'GND',
};

export const KIND_LABELS: Record<EditorKind, TextKey> = {
  resistor: 'kind.resistor',
  wire: 'kind.wire',
  voltageSource: 'kind.voltageSource',
  currentSource: 'kind.currentSource',
  potentiometer: 'kind.potentiometer',
  powerLoad: 'kind.powerLoad',
  ground: 'kind.ground',
};

/** Half the length of a part along its axis, so its terminals are four grid steps apart. */
export const HALF = 2 * GRID;

const LOCAL_TERMINALS: Record<Exclude<EditorKind, 'wire'>, Partial<Record<Terminal, Point>>> = {
  resistor: { a: { x: -HALF, y: 0 }, b: { x: HALF, y: 0 } },
  voltageSource: { a: { x: -HALF, y: 0 }, b: { x: HALF, y: 0 } },
  currentSource: { a: { x: -HALF, y: 0 }, b: { x: HALF, y: 0 } },
  powerLoad: { a: { x: -HALF, y: 0 }, b: { x: HALF, y: 0 } },
  potentiometer: { a: { x: -HALF, y: 0 }, b: { x: HALF, y: 0 }, wiper: { x: 0, y: -HALF } },
  ground: { ground: { x: 0, y: 0 } },
};

export function snap(value: number): number {
  return Math.round(value / GRID) * GRID;
}

export function rotate(point: Point, quarterTurns: number): Point {
  const turns = ((quarterTurns % 4) + 4) % 4;
  switch (turns) {
    case 1:
      return { x: -point.y, y: point.x };
    case 2:
      return { x: -point.x, y: -point.y };
    case 3:
      return { x: point.y, y: -point.x };
    default:
      return point;
  }
}

/** The terminals of an element at their places on the canvas. */
export function terminals(element: EditorElement): Partial<Record<Terminal, Point>> {
  if (element.kind === 'wire') {
    return {
      a: { x: element.x, y: element.y },
      b: { x: element.x2 ?? element.x, y: element.y2 ?? element.y },
    };
  }
  const placed: Partial<Record<Terminal, Point>> = {};
  for (const [name, local] of Object.entries(LOCAL_TERMINALS[element.kind])) {
    const turned = rotate(local, element.rotation);
    placed[name as Terminal] = { x: element.x + turned.x, y: element.y + turned.y };
  }
  return placed;
}

export const pointKey = (point: Point) => `${point.x},${point.y}`;

export interface Nodes {
  /** Node number by grid point; 0 is the ground. */
  byPoint: Map<string, number>;
  /** One representative point per node, where its voltage is shown. */
  pointOf: Map<number, Point>;
  /** Whether the node was chosen by a ground symbol rather than picked for lack of one. */
  grounded: boolean;
}

/** A wire without a resistance: no `rOverride` and not all of material, length and area. */
export function isIdealWire(element: EditorElement): boolean {
  const given = (key: keyof EditorElement) => {
    const value = element[key];
    return typeof value === 'string' ? value.trim() !== '' : value != null;
  };
  return element.kind === 'wire' && !given('rOverride') && !(given('material') && given('length') && given('area'));
}

/** Whether `p` lies on the segment from `a` to `b` but on neither end. */
function inside(a: Point, b: Point, p: Point): boolean {
  if ((b.x - a.x) * (p.y - a.y) !== (b.y - a.y) * (p.x - a.x)) return false;
  if (pointKey(p) === pointKey(a) || pointKey(p) === pointKey(b)) return false;
  return Math.min(a.x, b.x) <= p.x && p.x <= Math.max(a.x, b.x) && Math.min(a.y, b.y) <= p.y && p.y <= Math.max(a.y, b.y);
}

/**
 * The points where a terminal or wire end is connected: where it meets another one, or sits in the
 * middle of an ideal wire. The editor draws the open dot only at the others.
 */
export function connectedPoints(elements: EditorElement[]): Set<string> {
  const count = new Map<string, number>();
  const points: Point[] = [];
  for (const element of elements) {
    for (const point of Object.values(terminals(element)) as Point[]) {
      const key = pointKey(point);
      count.set(key, (count.get(key) ?? 0) + 1);
      points.push(point);
    }
  }
  const connected = new Set([...count].filter(([, n]) => n > 1).map(([key]) => key));
  for (const wire of elements.filter(isIdealWire)) {
    const { a, b } = terminals(wire);
    for (const point of points) if (inside(a!, b!, point)) connected.add(pointKey(point));
  }
  return connected;
}

/**
 * Numbers the points where terminals sit. Ground is the point of a ground symbol; without one it
 * is the negative terminal of the first voltage source, so a drawn source-and-load circuit
 * works without ceremony. A terminal in the middle of an ideal wire is a junction and shares the
 * node of the wire's start; a resistive wire is joined only at its ends.
 */
export function numberNodes(elements: EditorElement[]): Nodes {
  const points = new Map<string, Point>();
  let ground: string | null = null;
  for (const element of elements) {
    for (const [name, point] of Object.entries(terminals(element))) {
      const key = pointKey(point);
      if (!points.has(key)) points.set(key, point);
      if (element.kind === 'ground' && name === 'ground' && ground === null) ground = key;
    }
  }
  const grounded = ground !== null;
  if (ground === null) {
    const source = elements.find((e) => e.kind === 'voltageSource');
    if (source) ground = pointKey(terminals(source).b!);
  }

  // Junctions: each point inside an ideal wire stands for the wire's start.
  const alias = new Map<string, string>();
  for (const wire of elements.filter(isIdealWire)) {
    const { a, b } = terminals(wire);
    for (const [key, point] of points) {
      if (!alias.has(key) && inside(a!, b!, point)) alias.set(key, pointKey(a!));
    }
  }
  const canonical = (key: string) => {
    const seen = new Set<string>();
    while (alias.has(key) && !seen.has(key)) {
      seen.add(key);
      key = alias.get(key)!;
    }
    return key;
  };

  const byPoint = new Map<string, number>();
  const pointOf = new Map<number, Point>();
  const numberOf = new Map<string, number>();
  if (ground !== null) {
    numberOf.set(canonical(ground), 0);
    pointOf.set(0, points.get(ground)!);
  }
  let next = 1;
  for (const [key, point] of points) {
    const root = canonical(key);
    if (!numberOf.has(root)) {
      numberOf.set(root, next);
      pointOf.set(next, point);
      next += 1;
    }
    byPoint.set(key, numberOf.get(root)!);
  }
  return { byPoint, pointOf, grounded };
}

/** The editor-only fields never go to the solver. */
const EDITOR_FIELDS = ['kind', 'x', 'y', 'rotation', 'x2', 'y2', 'reversed'] as const;

/** The ID and the parameters the user typed; blank ones are left out, as missing. */
function parametersOf(element: EditorElement): Record<string, unknown> {
  const parameters: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(element)) {
    if ((EDITOR_FIELDS as readonly string[]).includes(key)) continue;
    if (typeof value === 'string' && value.trim() === '') continue;
    parameters[key] = value;
  }
  return parameters;
}

export function toRequest(elements: EditorElement[], nodes: Nodes, precision?: number | null): CircuitRequest {
  const node = (point: Point | undefined) => (point ? (nodes.byPoint.get(pointKey(point)) ?? 0) : 0);
  const list: CircuitElement[] = [];
  for (const element of elements) {
    if (element.kind === 'ground') continue;
    const places = terminals(element);
    const parameters = parametersOf(element);
    const entry = { ...parameters, kind: element.kind, a: node(places.a), b: node(places.b) } as CircuitElement;
    if (places.wiper) entry.wiper = node(places.wiper);
    list.push(entry);
  }
  const ids = [...nodes.pointOf.keys()].sort((p, q) => p - q);
  return { nodes: ids.length ? ids : [0], elements: list, precision: precision ?? null };
}

/**
 * The Kirchhoff mode works the nodes out from the drawing itself, so it gets the terminals in grid
 * steps instead of node numbers. Ground symbols mean nothing there and stay behind.
 */
export function toKirchhoffRequest(elements: EditorElement[]): KirchhoffRequest {
  const grid = (point: Point) => ({ x: point.x / GRID, y: point.y / GRID });
  const list: KirchhoffElement[] = [];
  for (const element of elements) {
    if (element.kind === 'ground') continue;
    const { a, b } = terminals(element);
    const entry = { ...parametersOf(element), kind: element.kind, pa: grid(a!), pb: grid(b!) } as KirchhoffElement;
    if (element.reversed) entry.reversed = true;
    list.push(entry);
  }
  return { elements: list };
}

/**
 * Where a junction's dot belongs: of the points joined to `anchor` by ideal wires, the one where
 * the most terminals and wires meet, the first in reading order of those. The engine names a node
 * by its first point, which may be the end of a part rather than the fork of the wires.
 */
export function junctionPoint(elements: EditorElement[], nodes: Nodes, anchor: Point): Point {
  // `nodes` keeps the two ends of a wire apart, as the electronics mode needs; here they are one.
  const parent = new Map<number, number>();
  const root = (n: number): number => {
    while (parent.has(n)) n = parent.get(n)!;
    return n;
  };
  for (const wire of elements.filter(isIdealWire)) {
    const { a, b } = terminals(wire);
    const [p, q] = [nodes.byPoint.get(pointKey(a!)), nodes.byPoint.get(pointKey(b!))];
    if (p !== undefined && q !== undefined && root(p) !== root(q)) parent.set(root(p), root(q));
  }
  const nodeOf = (point: Point) => {
    const n = nodes.byPoint.get(pointKey(point));
    return n === undefined ? undefined : root(n);
  };
  const node = nodeOf(anchor);
  if (node === undefined) return anchor;
  const degree = new Map<string, number>();
  const count = (point: Point, by: number) => degree.set(pointKey(point), (degree.get(pointKey(point)) ?? 0) + by);
  const unique = new Map<string, Point>();
  for (const element of elements) {
    if (element.kind === 'ground') continue;
    for (const point of Object.values(terminals(element)) as Point[]) {
      count(point, 1);
      unique.set(pointKey(point), point);
    }
  }
  const points = [...unique.values()];
  // A point in the middle of an ideal wire has the wire on both sides.
  for (const wire of elements.filter(isIdealWire)) {
    const { a, b } = terminals(wire);
    for (const point of points) if (inside(a!, b!, point)) count(point, 2);
  }
  let best = anchor;
  for (const point of points) {
    if (nodeOf(point) !== node) continue;
    const [d, e] = [degree.get(pointKey(point))!, degree.get(pointKey(best)) ?? 0];
    if (d > e || (d === e && (point.y < best.y || (point.y === best.y && point.x < best.x)))) best = point;
  }
  return best;
}

/** The next unused ID of a kind: R1, R2, … */
export function freshId(elements: EditorElement[], kind: EditorKind): string {
  const taken = new Set(elements.map((e) => e.id));
  for (let n = 1; ; n += 1) {
    const id = `${PREFIX[kind]}${n}`;
    if (!taken.has(id)) return id;
  }
}

export function newElement(elements: EditorElement[], kind: EditorKind, at: Point): EditorElement {
  const base = { id: freshId(elements, kind), kind, x: snap(at.x), y: snap(at.y), rotation: 0 };
  return kind === 'wire' ? { ...base, x2: snap(at.x) + 2 * HALF, y2: snap(at.y) } : base;
}

/** The results a part can show on the canvas, with their symbols. */
export const QUANTITIES = [
  { key: 'voltage', symbol: 'U', label: 'result.voltage' },
  { key: 'current', symbol: 'I', label: 'result.current' },
  { key: 'power', symbol: 'P', label: 'result.power' },
  { key: 'resistance', symbol: 'R', label: 'result.resistance' },
] as const satisfies readonly { key: string; symbol: string; label: TextKey }[];

export type Quantity = (typeof QUANTITIES)[number]['key'];

/** Which results the canvas shows; voltage and current until changed. */
export type Shown = Record<Quantity, boolean>;
export const SHOWN: Shown = { voltage: true, current: true, power: false, resistance: false };

/** The text fields the inspector offers per kind; `{,}` in a placeholder is the decimal mark. */
export interface Field {
  key: keyof CircuitElement;
  label: TextKey;
  placeholder: string;
}

export const FIELDS: Record<EditorKind, Field[]> = {
  resistor: [{ key: 'r', label: 'field.r', placeholder: '10\\Omega' }],
  wire: [
    { key: 'length', label: 'field.length', placeholder: '100m' },
    { key: 'area', label: 'field.area', placeholder: '2mm^2' },
    { key: 'temp', label: 'field.temp', placeholder: '20°C' },
    { key: 'rOverride', label: 'field.rOverride', placeholder: '0{,}1\\Omega' },
  ],
  voltageSource: [
    { key: 'u', label: 'field.u', placeholder: '230V' },
    { key: 'internalResistance', label: 'field.internalResistance', placeholder: '0{,}5\\Omega' },
  ],
  currentSource: [
    { key: 'i', label: 'field.i', placeholder: '2A' },
    { key: 'internalConductance', label: 'field.internalConductance', placeholder: '1mS' },
  ],
  potentiometer: [
    { key: 'rTotal', label: 'field.rTotal', placeholder: '10k\\Omega' },
    { key: 'position', label: 'field.position', placeholder: '0{,}25' },
  ],
  powerLoad: [{ key: 'p', label: 'field.p', placeholder: '500W' }],
  ground: [],
};

/** The Kirchhoff mode's wires are ideal, so they have no fields. */
export const IDEAL_FIELDS: Record<EditorKind, Field[]> = { ...FIELDS, wire: [] };

export const MATERIALS: { id: string; label: TextKey }[] = [
  { id: 'copper', label: 'material.copper' },
  { id: 'aluminium', label: 'material.aluminium' },
  { id: 'silver', label: 'material.silver' },
  { id: 'gold', label: 'material.gold' },
  { id: 'iron', label: 'material.iron' },
  { id: 'constantan', label: 'material.constantan' },
  { id: 'brass', label: 'material.brass' },
];

/** The version of the circuit file; the backend refuses any other. */
export const FILE_VERSION = 1;

/**
 * The file holds the circuit as the editor keeps it, in the structure of a `CircuitRequest`
 * (nodes, elements with their parameter texts, node numbers filled in) plus the editor's
 * positions and a version.
 */
export function toFile(elements: EditorElement[]) {
  const nodes = numberNodes(elements);
  const request = toRequest(elements, nodes);
  const placed = new Map(elements.map((e) => [e.id, e]));
  const solverElements = new Map(request.elements.map((e) => [e.id, e]));
  return {
    version: FILE_VERSION,
    nodes: request.nodes,
    elements: elements.map((element) => {
      // Ground has no place in a request, but the file keeps it like any other part.
      const entry = solverElements.get(element.id) ?? { a: 0, b: 0 };
      return { ...entry, ...placed.get(element.id), a: entry.a, b: entry.b };
    }),
  };
}

/** The editor's elements from a file the backend has already checked. */
export function fromFile(file: unknown): EditorElement[] {
  const { elements } = file as { elements: (EditorElement & { a?: number; b?: number; wiper?: number })[] };
  // Node numbers are derived from the positions again, so they are not carried over.
  return elements.map(({ a: _a, b: _b, wiper: _wiper, rotation, ...rest }) => ({
    ...rest,
    rotation: typeof rotation === 'number' ? rotation : 0,
  }));
}
