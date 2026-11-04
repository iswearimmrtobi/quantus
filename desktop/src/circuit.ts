import type { TextKey } from './i18n/en';
import type { CircuitElement, CircuitRequest, ElementKind, KirchhoffElement, KirchhoffRequest, MergeStep } from './api';

/**
 * The schematic as the editor keeps it, and the pure functions that turn it into a
 * `CircuitRequest`. Nodes are never typed in: two terminals are connected when they sit on the
 * same grid point, or one sits in the middle of an ideal wire, so the numbers the solver needs
 * come out of the geometry.
 */

export const GRID = 20;

/**
 * `ground` is the reference potential (node 0); the solver never sees it as an element. A
 * `terminal` is A or B of the Kirchhoff mode's resistance between two points, likewise no part.
 */
export type EditorKind = ElementKind | 'ground' | 'terminal';

/**
 * `a`, `b` and `wiper` are filled in by [`toRequest`]; what the editor stores is the position.
 * A wire runs from (x, y) to (x2, y2); every other part sits at (x, y), turned by
 * `rotation` quarter turns. A resistor may also sit at 45°: then `rotation` ends in a half.
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

export type Terminal = 'a' | 'b' | 'wiper' | 'ground' | 'terminal';

const PREFIX: Record<EditorKind, string> = {
  resistor: 'R',
  wire: 'W',
  voltageSource: 'V',
  currentSource: 'I',
  potentiometer: 'P',
  powerLoad: 'L',
  ground: 'GND',
  terminal: 'T',
};

export const KIND_LABELS: Record<EditorKind, TextKey> = {
  resistor: 'kind.resistor',
  wire: 'kind.wire',
  voltageSource: 'kind.voltageSource',
  currentSource: 'kind.currentSource',
  potentiometer: 'kind.potentiometer',
  powerLoad: 'kind.powerLoad',
  ground: 'kind.ground',
  terminal: 'kind.terminal',
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
  terminal: { terminal: { x: 0, y: 0 } },
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

/** Whether a part sits at 45° to the grid: its `rotation` ends in a half, 0.5 being 45°. */
export const isDiagonal = (rotation: number) => !Number.isInteger(rotation);

/**
 * A resistor's turns in eighths: by 45° about its middle, or by a quarter. Only a resistor turns
 * by 45°; its terminals then sit two grid steps across and two down from the middle, so they stay
 * on the grid, and the symbol gets longer leads.
 */
export const turnedBy = (rotation: number, eighths: number) => (((rotation + eighths / 2) % 4) + 4) % 4;

/** The terminals of an element at their places on the canvas. */
export function terminals(element: EditorElement): Partial<Record<Terminal, Point>> {
  if (element.kind === 'wire') {
    return {
      a: { x: element.x, y: element.y },
      b: { x: element.x2 ?? element.x, y: element.y2 ?? element.y },
    };
  }
  const diagonal = isDiagonal(element.rotation);
  const placed: Partial<Record<Terminal, Point>> = {};
  for (const [name, local] of Object.entries(LOCAL_TERMINALS[element.kind])) {
    // At 45° the point turns by an eighth and grows by √2, which keeps it on the grid.
    const start = diagonal ? { x: local.x - local.y, y: local.x + local.y } : local;
    const turned = rotate(start, Math.floor(element.rotation));
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
export function inside(a: Point, b: Point, p: Point): boolean {
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
 * The points where three or more lines meet: terminals and wire ends, a point in the middle of an
 * ideal wire counting twice. That is where a schematic puts a filled junction dot; a corner of
 * two lines gets none. The terminals A and B are marks, not lines, and do not count.
 */
export function junctionDots(elements: EditorElement[]): Point[] {
  const count = new Map<string, number>();
  const points = new Map<string, Point>();
  const add = (point: Point, by: number) => {
    const key = pointKey(point);
    count.set(key, (count.get(key) ?? 0) + by);
    points.set(key, point);
  };
  const lines = elements.filter((e) => e.kind !== 'terminal');
  for (const element of lines) {
    for (const point of Object.values(terminals(element)) as Point[]) add(point, 1);
  }
  const ends = [...points.values()];
  for (const wire of lines.filter(isIdealWire)) {
    const { a, b } = terminals(wire);
    for (const point of ends) if (inside(a!, b!, point)) add(point, 2);
  }
  return [...points].filter(([key]) => count.get(key)! >= 3).map(([, point]) => point);
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
    if (element.kind === 'terminal') continue;
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
    if (element.kind === 'ground' || element.kind === 'terminal') continue;
    const places = terminals(element);
    const parameters = parametersOf(element);
    const entry = { ...parameters, kind: element.kind, a: node(places.a), b: node(places.b) } as CircuitElement;
    if (places.wiper) entry.wiper = node(places.wiper);
    list.push(entry);
  }
  const ids = [...nodes.pointOf.keys()].sort((p, q) => p - q);
  return { nodes: ids.length ? ids : [0], elements: list, precision: precision ?? null };
}

/** The terminals A and B of the drawing, in that order: by ID, so A before B. */
export function terminalParts(elements: EditorElement[]): EditorElement[] {
  return elements.filter((e) => e.kind === 'terminal').sort((p, q) => p.id.localeCompare(q.id));
}

/**
 * The Kirchhoff mode works the nodes out from the drawing itself, so it gets the terminals in grid
 * steps instead of node numbers. Ground symbols mean nothing there and stay behind. With both
 * terminals A and B placed, the request asks for the resistance between them.
 */
export function toKirchhoffRequest(elements: EditorElement[]): KirchhoffRequest {
  const grid = (point: Point) => ({ x: point.x / GRID, y: point.y / GRID });
  const list: KirchhoffElement[] = [];
  for (const element of elements) {
    if (element.kind === 'ground' || element.kind === 'terminal') continue;
    const { a, b } = terminals(element);
    const entry = { ...parametersOf(element), kind: element.kind, pa: grid(a!), pb: grid(b!) } as KirchhoffElement;
    if (element.reversed) entry.reversed = true;
    list.push(entry);
  }
  const ends = terminalParts(elements);
  if (ends.length === 2) return { elements: list, terminals: [grid(ends[0]), grid(ends[1])] };
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
    if (element.kind === 'ground' || element.kind === 'terminal') continue;
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

/** The next unused ID of a kind: R1, R2, …; the terminals are A and B. */
export function freshId(elements: EditorElement[], kind: EditorKind): string {
  const taken = new Set(elements.map((e) => e.id));
  if (kind === 'terminal') {
    const free = ['A', 'B'].find((id) => !taken.has(id));
    if (free) return free;
  }
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
  terminal: [],
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

export type { MergeStep };

/** `R_{23}` as `R23`, `R_{\mathrm{X},2}` as `RX,2`, `R_{\Delta 1}` as `RΔ1`: the name a merged resistor shows on the canvas. */
const plainSymbol = (symbol: string) =>
  symbol
    .replace(/\\mathrm/g, '')
    .replace(/\\Delta\s?/g, 'Δ')
    .replace(/[_{}]/g, '');

/**
 * The eight ways a resistor can lie, from `a` to `b`, with its rotation: a quarter turn is 1,
 * 45° is 0.5. A resistor lying along `d` reaches from a point `p` to `p + 2·HALF·d`.
 */
const DIRECTIONS = [
  { x: 1, y: 0, rotation: 0 },
  { x: 1, y: 1, rotation: 0.5 },
  { x: 0, y: 1, rotation: 1 },
  { x: -1, y: 1, rotation: 1.5 },
  { x: -1, y: 0, rotation: 2 },
  { x: -1, y: -1, rotation: 2.5 },
  { x: 0, y: -1, rotation: 3 },
  { x: 1, y: -1, rotation: 3.5 },
];

/** How far the preview looks for room for a new resistor, in grid steps. */
const SEARCH = 6;
/** The offsets searched, nearest first, so that a good layout is found early and the rest skipped. */
const OFFSETS = Array.from({ length: (2 * SEARCH + 1) ** 2 }, (_, n) => ({ dx: (n % (2 * SEARCH + 1)) - SEARCH, dy: Math.floor(n / (2 * SEARCH + 1)) - SEARCH })).sort(
  (p, q) => Math.abs(p.dx) + Math.abs(p.dy) - (Math.abs(q.dx) + Math.abs(q.dy)),
);
/** What touching the drawing costs a layout, against one grid step of wire. */
const CLASH = 1000;

/** The grid points from `p` to `q` on a straight line, across, down or at 45°, both ends included. */
function straight(p: Point, q: Point): Point[] {
  const steps = Math.max(Math.abs(q.x - p.x), Math.abs(q.y - p.y)) / GRID;
  return Array.from({ length: steps + 1 }, (_, k) => ({ x: p.x + ((q.x - p.x) * k) / (steps || 1), y: p.y + ((q.y - p.y) * k) / (steps || 1) }));
}

/** The corner of a wire from `p` to `q` as the editor draws it, across first or down first. */
const bend = (p: Point, q: Point, acrossFirst: boolean): Point => (acrossFirst ? { x: q.x, y: p.y } : { x: p.x, y: q.y });

/** The grid points of a wire from `p` to `q`, across and down. */
const wirePath = (p: Point, q: Point, acrossFirst: boolean) => {
  const turn = bend(p, q, acrossFirst);
  return [...straight(p, turn), ...straight(turn, q).slice(1)];
};

/** The wires from `p` to `q`: none, one, or two at a right angle. */
function wiresOf(id: string, p: Point, q: Point, acrossFirst: boolean): EditorElement[] {
  const turn = bend(p, q, acrossFirst);
  const wire = (key: string, from: Point, to: Point): EditorElement => ({ id: `~${id}${key}`, kind: 'wire', x: from.x, y: from.y, x2: to.x, y2: to.y, rotation: 0 });
  return [
    ...(pointKey(p) === pointKey(turn) ? [] : [wire('', p, turn)]),
    ...(pointKey(turn) === pointKey(q) ? [] : [wire('+', turn, q)]),
  ];
}

/**
 * How many points of `path` touch something they must not: what is drawn (`taken`) other than the
 * points it is meant to join (`ends`), or the path itself a second time. Each would show as a
 * connection that is not there, or hide one.
 */
function clashes(path: Point[], taken: Set<number>, ends: Set<number>): number {
  const seen = new Set<number>();
  let count = 0;
  for (const point of path) {
    const key = cell(point);
    if (seen.has(key) || (!ends.has(key) && taken.has(key))) count++;
    seen.add(key);
  }
  return count;
}

/** A grid point as one number, for the sets the layout searches through. */
const cell = (p: Point) => Math.round(p.x / GRID) * 65536 + Math.round(p.y / GRID);

const steps = (p: Point, q: Point) => (Math.abs(p.x - q.x) + Math.abs(p.y - q.y)) / GRID;

/** A new resistor as drawn: the resistor, the wires that lead to it, and every grid point they cover. */
interface Placed {
  resistor: EditorElement;
  wires: EditorElement[];
  path: Point[];
}

/**
 * A resistor between the drawn points `from` and `to`: of the places near the middle and the eight
 * directions, the one that needs the least wire without touching anything else on the way.
 */
function placeBetween(id: string, from: Point, to: Point, taken: Set<number>): Placed {
  const ends = new Set([cell(from), cell(to)]);
  const middle = { x: snap((from.x + to.x) / 2), y: snap((from.y + to.y) / 2) };
  let best: (Placed & { cost: number }) | null = null;
  for (const { dx, dy } of OFFSETS) {
    {
      const centre = { x: middle.x + dx * GRID, y: middle.y + dy * GRID };
      for (const d of DIRECTIONS) {
        const resistor: EditorElement = { id, kind: 'resistor', x: centre.x, y: centre.y, rotation: d.rotation };
        const { a, b } = terminals(resistor) as Record<'a' | 'b', Point>;
        for (const [near, far] of [[a, b], [b, a]]) {
          // The wire it needs at least; a layout that cannot beat the best one is not built.
          if (best && steps(from, near) + steps(far, to) >= best.cost) continue;
          for (const first of [true, false]) {
            for (const second of [true, false]) {
              const path = [...wirePath(from, near, first), ...straight(near, far).slice(1), ...wirePath(far, to, second).slice(1)];
              const cost =
                steps(from, near) + steps(far, to) + CLASH * clashes(path, taken, ends) + (Math.abs(dx) + Math.abs(dy)) / 100;
              if (!best || cost < best.cost) {
                best = { cost, resistor, path, wires: [...wiresOf(`${id}a`, from, near, first), ...wiresOf(`${id}b`, far, to, second)] };
              }
            }
          }
        }
      }
    }
  }
  return best!;
}

/**
 * The arms of a new star: each a resistor from the centre in its own one of the eight directions,
 * with wires on to its corner. The centre is the free grid point near `centre` from which the
 * arms reach their corners with the least wire, touching nothing else on the way nor each other.
 */
function placeStar(arms: { id: string; corner: Point }[], centre: Point, taken: Set<number>): { centre: Point; arms: Placed[] } {
  const reach = 2 * HALF;
  let best: { cost: number; centre: Point; arms: Placed[] } | null = null;
  for (const { dx, dy } of OFFSETS) {
    {
      const c = { x: centre.x + dx * GRID, y: centre.y + dy * GRID };
      if (taken.has(cell(c))) continue;
      // The wire the arms need at least, each in its nearest direction; no better than the best, skip.
      const reachable = (corner: Point) =>
        Math.min(...DIRECTIONS.map((d) => steps({ x: c.x + d.x * reach, y: c.y + d.y * reach }, corner)));
      if (best && arms.reduce((sum, arm) => sum + reachable(arm.corner), 0) >= best.cost) continue;
      // The best arm in each direction for each corner, on its own.
      const options = arms.map((arm) =>
        DIRECTIONS.map((d) => {
          const end = { x: c.x + d.x * reach, y: c.y + d.y * reach };
          const resistor: EditorElement = { id: arm.id, kind: 'resistor', x: c.x + d.x * HALF, y: c.y + d.y * HALF, rotation: d.rotation };
          const ends = new Set([cell(c), cell(arm.corner)]);
          return [true, false]
            .map((first) => {
              const path = [...straight(c, end), ...wirePath(end, arm.corner, first).slice(1)];
              return { cost: steps(end, arm.corner) + CLASH * clashes(path, taken, ends), placed: { resistor, path, wires: wiresOf(arm.id, end, arm.corner, first) } };
            })
            .reduce((p, q) => (q.cost < p.cost ? q : p));
        }),
      );
      for (let i = 0; i < 8; i++) {
        for (let j = 0; j < 8; j++) {
          for (let k = 0; k < 8; k++) {
            if (i === j || j === k || i === k) continue;
            const chosen = [options[0][i], options[1][j], options[2][k]];
            // Arms that share a point other than the centre cross or overlap.
            const base = chosen.reduce((sum, o) => sum + o.cost, 0);
            if (best && base >= best.cost) continue;
            const seen = new Set<number>();
            let shared = 0;
            for (const option of chosen) {
              for (const point of option.placed.path.slice(1)) {
                const key = cell(point);
                if (seen.has(key)) shared++;
                seen.add(key);
              }
            }
            const cost = base + CLASH * shared + (Math.abs(dx) + Math.abs(dy)) / 100;
            if (!best || cost < best.cost) best = { cost, centre: c, arms: chosen.map((o) => o.placed) };
          }
        }
      }
    }
  }
  return best!;
}

/**
 * The drawing after the merges up to and including `upTo`, drawn from the original one: a merged
 * resistor takes the place of the first resistor it stands for, under its new name. Of the others,
 * one in series becomes a wire, since its current runs on through the merged one; one in parallel
 * goes, and the wires that led only to it go with it. A star–delta conversion takes out its three
 * resistors and draws the three new ones where they join their nodes without touching anything
 * else. `merged` holds the IDs of the last step's resistors, to mark them.
 */
export function reducedDrawing(elements: EditorElement[], merges: MergeStep[], upTo: number): { elements: EditorElement[]; merged: string[] } {
  return reducedDrawings(elements, merges.slice(0, upTo + 1))[upTo];
}

/** [`reducedDrawing`] for every step at once, in one pass: what the hover shows, step by step. */
export function reducedDrawings(elements: EditorElement[], merges: MergeStep[]): { elements: EditorElement[]; merged: string[] }[] {
  const shown: { elements: EditorElement[]; merged: string[] }[] = [];
  const drawn = new Map(elements.map((e) => [e.id, e]));
  /** The resistors drawn now, by the ID of the part whose place they take, with the parts they stand for. */
  const groups = new Map<string, string[]>(elements.filter((e) => e.kind === 'resistor').map((e) => [e.id, [e.id]]));
  /** The wires drawn for a resistor of the preview: they go when it goes. */
  const feeders = new Map<string, string[]>();
  /** Where the preview drew the centre of a new star, by the engine's grid point for it. */
  const moved = new Map<string, Point>();
  const at = (p: Point) => moved.get(pointKey(p)) ?? { x: p.x * GRID, y: p.y * GRID };
  const dropWire = (id: string) => {
    for (const key of [...drawn.keys()]) if (key === id || key.startsWith(`${id}#`)) drawn.delete(key);
  };
  const drop = (place: string) => {
    drawn.delete(place);
    groups.delete(place);
    for (const wire of feeders.get(place) ?? []) dropWire(wire);
    feeders.delete(place);
  };
  let merged: string[] = [];
  for (const merge of merges) {
    const members = [...groups].filter(([, parts]) => parts.every((p) => merge.parts.includes(p)));
    if (merge.created.length > 0) {
      for (const [place] of members) drop(place);
      // The wires that led only to the three go now, so the new ones are placed in the room they
      // leave; those to the points where the new ones attach stay.
      const anchors = merge.created.flatMap((c) => (merge.kind === 'deltaToStar' ? [at(c.a)] : [at(c.a), at(c.b)]));
      const pruned = withoutLooseWires([...drawn.values()], anchors);
      for (const [key, element] of [...drawn]) if (element.kind === 'wire') drawn.delete(key);
      for (const element of pruned) if (element.kind === 'wire') drawn.set(element.id, element);

      const taken = new Set([...drawn.values()].flatMap((e) => coveredPoints(e).map(cell)));
      const names = merge.created.map((created) => plainSymbol(created.symbol));
      let placed: Placed[];
      if (merge.kind === 'deltaToStar') {
        // A new star shares its centre `b`.
        const star = placeStar(
          merge.created.map((c, k) => ({ id: names[k], corner: at(c.a) })),
          at(merge.created[0].b),
          taken,
        );
        moved.set(pointKey(merge.created[0].b), star.centre);
        placed = star.arms;
      } else {
        placed = merge.created.map((c, k) => {
          const side = placeBetween(names[k], at(c.a), at(c.b), taken);
          for (const point of side.path) taken.add(cell(point));
          return side;
        });
      }
      merge.created.forEach((created, k) => {
        drawn.set(created.id, placed[k].resistor);
        for (const wire of placed[k].wires) drawn.set(wire.id, wire);
        groups.set(created.id, [created.id]);
        feeders.set(created.id, placed[k].wires.map((w) => w.id));
      });
      merged = names;
      shown.push({ elements: withoutLooseWires([...drawn.values()]), merged });
      continue;
    }
    members.sort(([, p], [, q]) => merge.parts.indexOf(p[0]) - merge.parts.indexOf(q[0]));
    if (members.length < 2) {
      shown.push({ elements: withoutLooseWires([...drawn.values()]), merged });
      continue;
    }
    const [keep] = members[0];
    for (const [place] of members.slice(1)) {
      const element = drawn.get(place)!;
      if (merge.kind === 'series') {
        // Its current runs on through the merged one, so it stays as a wire, which goes with it.
        const { a, b } = terminals(element);
        const wire: EditorElement = { id: `~${place}`, kind: 'wire', x: a!.x, y: a!.y, x2: b!.x, y2: b!.y, rotation: 0 };
        drawn.delete(place);
        groups.delete(place);
        drawn.set(wire.id, wire);
        feeders.set(keep, [...(feeders.get(keep) ?? []), ...(feeders.get(place) ?? []), wire.id]);
        feeders.delete(place);
      } else {
        drop(place);
      }
    }
    groups.set(keep, members.flatMap(([, parts]) => parts));
    merged = [plainSymbol(merge.symbol)];
    drawn.set(keep, { ...drawn.get(keep)!, id: merged[0] });
    shown.push({ elements: withoutLooseWires([...drawn.values()]), merged });
  }
  return shown;
}

/**
 * The drawing without the wires that end in nothing: they led only to a part that went. A wire is
 * cut first where something meets it in its middle, so of a rail only the piece past the last part goes.
 * The `anchors` count as connected, for parts that are still to be drawn there.
 */
function withoutLooseWires(elements: EditorElement[], anchors: Point[] = []): EditorElement[] {
  let result = splitAtJunctions(elements, anchors);
  for (;;) {
    const count = new Map<string, number>(anchors.map((p) => [pointKey(p), 1]));
    for (const element of result) {
      for (const point of Object.values(terminals(element)) as Point[]) count.set(pointKey(point), (count.get(pointKey(point)) ?? 0) + 1);
    }
    const loose = result.find((e) => {
      if (e.kind !== 'wire') return false;
      const { a, b } = terminals(e);
      return count.get(pointKey(a!)) === 1 || count.get(pointKey(b!)) === 1;
    });
    if (!loose) return result;
    result = result.filter((e) => e !== loose);
  }
}

/** Every grid point a part or wire covers, from one end to the other, across, down or at 45°. */
function coveredPoints(element: EditorElement): Point[] {
  const { a, b } = terminals(element);
  if (!a || !b) return Object.values(terminals(element)) as Point[];
  const steps = Math.max(Math.abs(b.x - a.x), Math.abs(b.y - a.y)) / GRID;
  if (!Number.isInteger(steps) || (a.x !== b.x && a.y !== b.y && Math.abs(b.x - a.x) !== Math.abs(b.y - a.y))) return [a, b];
  return Array.from({ length: steps + 1 }, (_, k) => ({ x: a.x + ((b.x - a.x) * k) / steps, y: a.y + ((b.y - a.y) * k) / steps }));
}

/**
 * The drawing with every ideal wire cut into pieces where a terminal or wire end, or one of the
 * `extra` points, meets its middle.
 */
function splitAtJunctions(elements: EditorElement[], extra: Point[] = []): EditorElement[] {
  const points = [...elements.flatMap((e) => Object.values(terminals(e)) as Point[]), ...extra];
  return elements.flatMap((element) => {
    if (!isIdealWire(element)) return [element];
    const { a, b } = terminals(element) as Record<'a' | 'b', Point>;
    const taps = points.filter((p) => inside(a, b, p));
    if (taps.length === 0) return [element];
    const distance = (p: Point) => Math.abs(p.x - a.x) + Math.abs(p.y - a.y);
    const unique = [...new Map(taps.map((p) => [pointKey(p), p])).values()].sort((p, q) => distance(p) - distance(q));
    const stops = [a, ...unique, b];
    return stops.slice(1).map((to, k) => ({
      ...element,
      id: k === 0 ? element.id : `${element.id}#${k}`,
      x: stops[k].x,
      y: stops[k].y,
      x2: to.x,
      y2: to.y,
    }));
  });
}
