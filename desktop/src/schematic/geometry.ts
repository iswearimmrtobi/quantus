/**
 * What the schematic editors share, the electronics mode's and the Logic mode's: the grid, points
 * and boxes on it, turning and moving parts, and the connectivity of wires and pins. Everything is
 * in drawing coordinates; a wire runs from (x, y) to (x2, y2), every other part sits at (x, y),
 * turned by `rotation` quarter turns.
 */

export const GRID = 20;

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

/** Whether `p` lies on the segment from `a` to `b` but on neither end. */
export function inside(a: Point, b: Point, p: Point): boolean {
  if ((b.x - a.x) * (p.y - a.y) !== (b.y - a.y) * (p.x - a.x)) return false;
  if (pointKey(p) === pointKey(a) || pointKey(p) === pointKey(b)) return false;
  return Math.min(a.x, b.x) <= p.x && p.x <= Math.max(a.x, b.x) && Math.min(a.y, b.y) <= p.y && p.y <= Math.max(a.y, b.y);
}

/** Where a wire drawn from `from` to `to` turns: wires run only across and down, so a slant is an L. */
export const corner = (from: Point, to: Point, across: boolean | null): Point =>
  across === false ? { x: from.x, y: to.y } : { x: to.x, y: from.y };

/** The two ends of a wire. */
export const wireEnds = (wire: Placeable): [Point, Point] => [
  { x: wire.x, y: wire.y },
  { x: wire.x2 ?? wire.x, y: wire.y2 ?? wire.y },
];

/** A wire, as a segment that joins whatever sits on its ends or in its middle. */
export type Segment = [Point, Point];

/**
 * The points where a pin or wire end is connected: where it meets another one, or sits in the
 * middle of one of `joining`. `ends` holds every pin and wire end, each once per part. The editors
 * draw the open dot only at the others.
 */
export function connectedPointsOf(ends: Point[], joining: Segment[]): Set<string> {
  const count = new Map<string, number>();
  for (const point of ends) count.set(pointKey(point), (count.get(pointKey(point)) ?? 0) + 1);
  const connected = new Set([...count].filter(([, n]) => n > 1).map(([key]) => key));
  for (const [a, b] of joining) {
    for (const point of ends) if (inside(a, b, point)) connected.add(pointKey(point));
  }
  return connected;
}

/**
 * The points where three or more lines meet: pins and wire ends, a point in the middle of one of
 * `joining` counting twice. That is where a schematic puts a filled junction dot; a corner of two
 * lines gets none.
 */
export function junctionDotsOf(ends: Point[], joining: Segment[]): Point[] {
  const count = new Map<string, number>();
  const points = new Map<string, Point>();
  const add = (point: Point, by: number) => {
    const key = pointKey(point);
    count.set(key, (count.get(key) ?? 0) + by);
    points.set(key, point);
  };
  for (const point of ends) add(point, 1);
  const unique = [...points.values()];
  for (const [a, b] of joining) {
    for (const point of unique) if (inside(a, b, point)) add(point, 2);
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
    return o.kind === 'wire'
      ? { ...e, x: o.x + dx, y: o.y + dy, x2: (o.x2 ?? o.x) + dx, y2: (o.y2 ?? o.y) + dy }
      : { ...e, x: o.x + dx, y: o.y + dy };
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
    const start = turn(e.x, e.y);
    if (e.kind === 'wire') {
      const end = turn(e.x2 ?? e.x, e.y2 ?? e.y);
      return { ...e, x: start.x, y: start.y, x2: end.x, y2: end.y };
    }
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
