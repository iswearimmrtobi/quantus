/**
 * What the schematic editors share, the electronics mode's and the Logic mode's: the grid, points
 * and boxes on it, turning and moving parts, and the connectivity of wires and pins. Everything is
 * in drawing coordinates; a wire runs from (x, y) through its `bends` to (x2, y2), across and down;
 * every other part sits at (x, y), turned by `rotation` quarter turns.
 */

export const GRID = 20;

/** A side of a label's anchor. */
export type Side = 'below' | 'above' | 'right' | 'left';

export interface Point {
  x: number;
  y: number;
}

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

/** What the editors place: a part at a point, turned, or a wire (`kind` `wire`) between two points. */
export interface Placeable {
  id: string;
  kind: string;
  x: number;
  y: number;
  rotation: number;
  x2?: number;
  y2?: number;
  /** A wire's corners between its ends, in order from (x, y). */
  bends?: Point[];
}

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

export const pointKey = (point: Point) => `${point.x},${point.y}`;

/** Whether a part sits at 45° to the grid: its `rotation` ends in a half, 0.5 being 45°. */
export const isDiagonal = (rotation: number) => !Number.isInteger(rotation);

/** Where a pin `p` of a part's own coordinates lies on the canvas, the part turned and moved to its place. */
export function placeLocal(part: { x: number; y: number; rotation: number }, p: Point): Point {
  // At 45° the point turns by an eighth and grows by √2, which keeps it on the grid.
  const start = isDiagonal(part.rotation) ? { x: p.x - p.y, y: p.x + p.y } : p;
  const turned = rotate(start, Math.floor(part.rotation));
  return { x: part.x + turned.x, y: part.y + turned.y };
}

/**
 * The box around a rectangle of a part's own coordinates (two opposite `corners`), turned with the
 * part, exactly also at 45°, and moved to its place.
 */
export function turnedBox(part: { x: number; y: number; rotation: number }, corners: [Point, Point]): Rect {
  const diagonal = isDiagonal(part.rotation);
  // A rectangle turned by 45° is bounded by its four corners turned, not by the two given ones.
  const all = diagonal
    ? [corners[0], corners[1], { x: corners[0].x, y: corners[1].y }, { x: corners[1].x, y: corners[0].y }]
    : corners;
  const angle = (part.rotation * Math.PI) / 2;
  const turn = (p: Point) =>
    diagonal ? { x: p.x * Math.cos(angle) - p.y * Math.sin(angle), y: p.x * Math.sin(angle) + p.y * Math.cos(angle) } : rotate(p, part.rotation);
  return boundsOf(all.map(turn).map((p) => ({ x: part.x + p.x, y: part.y + p.y, w: 0, h: 0 })));
}

/** Whether `p` lies on the segment from `a` to `b` but on neither end. */
export function inside(a: Point, b: Point, p: Point): boolean {
  if ((b.x - a.x) * (p.y - a.y) !== (b.y - a.y) * (p.x - a.x)) return false;
  if (pointKey(p) === pointKey(a) || pointKey(p) === pointKey(b)) return false;
  return Math.min(a.x, b.x) <= p.x && p.x <= Math.max(a.x, b.x) && Math.min(a.y, b.y) <= p.y && p.y <= Math.max(a.y, b.y);
}

/** Where a wire drawn from `from` to `to` turns: wires run only across and down, so a slant is an L. */
export const corner = (from: Point, to: Point, across: boolean | null): Point =>
  across === false ? { x: from.x, y: to.y } : { x: to.x, y: from.y };

/** A wire from `from` to `to`. */
export const wireBetween = (id: string, from: Point, to: Point) => ({ id, kind: 'wire' as const, x: from.x, y: from.y, x2: to.x, y2: to.y, rotation: 0 });

/** The length of a wire put in from the palette: as long as a part with its leads. */
export const NEW_WIRE = 4 * GRID;

/** The two ends of a wire. */
export const wireEnds = (wire: Placeable): [Point, Point] => [
  { x: wire.x, y: wire.y },
  { x: wire.x2 ?? wire.x, y: wire.y2 ?? wire.y },
];

/** A straight piece of a wire. */
export type Segment = [Point, Point];

/** A wire's way from its start through its corners to its end: two points at least. */
export type Path = Point[];

/** A wire's points: its start, its corners, its end. */
export const wirePoints = (wire: Placeable): Path => {
  const [a, b] = wireEnds(wire);
  return [a, ...(wire.bends ?? []), b];
};

/** `wire` running along `path`; a straight one has no `bends`. */
export function withPath<E extends Placeable>(wire: E, path: Path): E {
  const { bends: _bends, ...rest } = wire;
  const [a, b] = [path[0], path[path.length - 1]];
  const placed = { ...rest, x: a.x, y: a.y, x2: b.x, y2: b.y } as E;
  return path.length > 2 ? { ...placed, bends: path.slice(1, -1) } : placed;
}

/** A wire's straight pieces, from its start. */
export const wireSegments = (wire: Placeable): Segment[] => {
  const path = wirePoints(wire);
  return path.slice(1).map((to, k): Segment => [path[k], to]);
};

/** A wire's longest straight piece, the first of equal ones: where its label and arrows go. */
export function longestPiece(wire: Placeable): Segment {
  const length = ([p, q]: Segment) => Math.abs(q.x - p.x) + Math.abs(q.y - p.y);
  return wireSegments(wire).reduce((best, piece) => (length(piece) > length(best) ? piece : best));
}

/** Whether `p` lies on `path` but on neither of its ends: in the middle of a piece or on a corner. */
export function onPath(path: Path, p: Point): boolean {
  const key = pointKey(p);
  if (key === pointKey(path[0]) || key === pointKey(path[path.length - 1])) return false;
  return path.some((q, k) => (k > 0 && k < path.length - 1 && pointKey(q) === key) || (k > 0 && inside(path[k - 1], q, p)));
}

/**
 * `path` without repeated points and without points in the middle of a straight run; a path that
 * goes nowhere keeps both its ends.
 */
export function simplify(path: Path): Path {
  const unique = path.filter((p, k) => k === 0 || pointKey(p) !== pointKey(path[k - 1]));
  if (unique.length < 2) return [unique[0], unique[0]];
  const kept = [unique[0]];
  for (let k = 1; k < unique.length - 1; k += 1) {
    const [prev, here, next] = [kept[kept.length - 1], unique[k], unique[k + 1]];
    const straight = (here.x - prev.x) * (next.y - here.y) === (here.y - prev.y) * (next.x - here.x);
    if (!straight) kept.push(here);
  }
  kept.push(unique[unique.length - 1]);
  return kept;
}

/**
 * A wire from `from` to `to` that turns halfway (on the grid), so that its stubs at both ends are
 * equal: sideways first when `across`, else downwards first. Points in line get a straight wire.
 */
export function route(from: Point, to: Point, across: boolean): Path {
  if (from.x === to.x || from.y === to.y) return [from, to];
  if (across) {
    const x = snap((from.x + to.x) / 2);
    return simplify([from, { x, y: from.y }, { x, y: to.y }, to]);
  }
  const y = snap((from.y + to.y) / 2);
  return simplify([from, { x: from.x, y }, { x: to.x, y }, to]);
}

/**
 * The wire a drag from `from` to `to` draws: between two pins one that turns halfway (`route`),
 * into the open an L whose corner is at the far end. `across` as for `corner`.
 */
export function drawnPath(from: Point, to: Point, across: boolean | null, betweenPins: boolean): Path {
  const sideways = across ?? Math.abs(to.x - from.x) >= Math.abs(to.y - from.y);
  return betweenPins ? route(from, to, sideways) : simplify([from, corner(from, to, sideways), to]);
}

/**
 * The points where a pin or wire end is connected: where it meets another one, or sits in the
 * middle of one of `joining`. `ends` holds every pin and wire end, each once per part. The editors
 * draw the open dot only at the others.
 */
export function connectedPointsOf(ends: Point[], joining: Path[]): Set<string> {
  const count = new Map<string, number>();
  for (const point of ends) count.set(pointKey(point), (count.get(pointKey(point)) ?? 0) + 1);
  const connected = new Set([...count].filter(([, n]) => n > 1).map(([key]) => key));
  for (const path of joining) {
    for (const point of ends) if (onPath(path, point)) connected.add(pointKey(point));
  }
  return connected;
}

/**
 * The points where three or more lines meet: pins and wire ends, a point in the middle of one of
 * `joining` counting twice. That is where a schematic puts a filled junction dot; a corner of two
 * lines gets none.
 */
export function junctionDotsOf(ends: Point[], joining: Path[]): Point[] {
  const count = new Map<string, number>();
  const points = new Map<string, Point>();
  const add = (point: Point, by: number) => {
    const key = pointKey(point);
    count.set(key, (count.get(key) ?? 0) + by);
    points.set(key, point);
  };
  for (const point of ends) add(point, 1);
  const unique = [...points.values()];
  for (const path of joining) {
    for (const point of unique) if (onPath(path, point)) add(point, 2);
  }
  return [...points].filter(([key]) => count.get(key)! >= 3).map(([, point]) => point);
}

/** A line as a thin box around it, `half` wider on every side. */
export function segmentBox(a: Point, b: Point, half: number): Rect {
  return {
    x: Math.min(a.x, b.x) - half,
    y: Math.min(a.y, b.y) - half,
    w: Math.abs(b.x - a.x) + 2 * half,
    h: Math.abs(b.y - a.y) + 2 * half,
  };
}

/** The smallest box around `rects`. */
export function boundsOf(rects: Rect[]): Rect {
  let [x0, y0, x1, y1] = [Infinity, Infinity, -Infinity, -Infinity];
  for (const r of rects) {
    x0 = Math.min(x0, r.x);
    y0 = Math.min(y0, r.y);
    x1 = Math.max(x1, r.x + r.w);
    y1 = Math.max(y1, r.y + r.h);
  }
  return { x: x0, y: y0, w: x1 - x0, h: y1 - y0 };
}

/** The parts with `origins` moved by (dx, dy) from where they were when the drag began. */
export function movedBy<E extends Placeable>(elements: E[], origins: Map<string, E>, dx: number, dy: number): E[] {
  return elements.map((e) => {
    const o = origins.get(e.id);
    if (!o) return e;
    return o.kind === 'wire' ? withPath(e, wirePoints(o).map((p) => ({ x: p.x + dx, y: p.y + dy }))) : { ...e, x: o.x + dx, y: o.y + dy };
  });
}

/**
 * The parts of `selection` turned by a quarter: one about itself (a wire about its start); several
 * together about the middle of the box around their `pins`, so they stay connected to each other.
 * `turned` gives a part's new rotation.
 */
export function rotatedQuarter<E extends Placeable>(
  elements: E[],
  selection: string[],
  pins: (e: E) => Point[],
  turned: (rotation: number) => number = (r) => (r + 1) % 4,
): E[] {
  const chosen = elements.filter((e) => selection.includes(e.id));
  if (chosen.length === 0) return elements;
  let center: Point;
  if (chosen.length === 1) {
    center = { x: chosen[0].x, y: chosen[0].y };
  } else {
    const points = chosen.flatMap(pins);
    const xs = points.map((p) => p.x);
    const ys = points.map((p) => p.y);
    center = { x: snap((Math.min(...xs) + Math.max(...xs)) / 2), y: snap((Math.min(...ys) + Math.max(...ys)) / 2) };
  }
  const turn = (x: number, y: number): Point => {
    const t = rotate({ x: x - center.x, y: y - center.y }, 1);
    return { x: center.x + t.x, y: center.y + t.y };
  };
  return elements.map((e) => {
    if (!selection.includes(e.id)) return e;
    if (e.kind === 'wire') return withPath(e, wirePoints(e).map((p) => turn(p.x, p.y)));
    const start = turn(e.x, e.y);
    return { ...e, x: start.x, y: start.y, rotation: turned(e.rotation) };
  });
}

/** The ids of the parts with their place or a pin inside the rectangle from `from` to `to`. */
export function inRectangle<E extends Placeable>(elements: E[], from: Point, to: Point, pins: (e: E) => Point[]): string[] {
  const [x0, x1] = [Math.min(from.x, to.x), Math.max(from.x, to.x)];
  const [y0, y1] = [Math.min(from.y, to.y), Math.max(from.y, to.y)];
  const within = (p: Point) => p.x >= x0 && p.x <= x1 && p.y >= y0 && p.y <= y1;
  return elements.filter((e) => [{ x: e.x, y: e.y }, ...pins(e)].some(within)).map((e) => e.id);
}

/** The next unused id `prefix1`, `prefix2`, … */
export function freshIdOf(taken: Iterable<string>, prefix: string): string {
  const used = new Set(taken);
  for (let n = 1; ; n += 1) {
    const id = `${prefix}${n}`;
    if (!used.has(id)) return id;
  }
}

/**
 * Where a part put in without a drop position goes: near the top left of what the canvas shows,
 * four to a row, `spacing` grid steps apart, after the `count` parts already there.
 */
export function freeSpot(view: { x: number; y: number; zoom: number }, count: number, spacing: number): Point {
  return {
    x: snap((160 - view.x) / view.zoom + (count % 4) * spacing * GRID),
    y: snap((120 - view.y) / view.zoom + Math.floor(count / 4) * spacing * GRID),
  };
}

/** The side of a label opposite `side`. */
const OPPOSITE: Record<Side, Side> = { below: 'above', above: 'below', right: 'left', left: 'right' };
export const opposite = (side: Side): Side => OPPOSITE[side];

/**
 * The order in which a label tries the sides of its anchor: `first`, then its opposite, then
 * `second` and its opposite; with `oppositeLast` the opposite of `first` comes last, as for a
 * label that must keep off the pin on that side.
 */
export function sidesAround(first: Side, second: Side, oppositeLast = false): Side[] {
  return oppositeLast ? [first, second, opposite(second), opposite(first)] : [first, opposite(first), second, opposite(second)];
}
