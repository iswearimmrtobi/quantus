import { GRID, HALF, rotate, terminals, type EditorElement, type Point } from './circuit';

/**
 * Places the labels on the circuit canvas so that none overlaps another label, a part or a wire.
 * Everything is in drawing coordinates, so zooming and moving the canvas leave the layout alone.
 */

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export type Side = 'below' | 'above' | 'right' | 'left';

export interface LabelRequest {
  id: string;
  /** The point the label belongs to; it sits `gap` away from it on one of `sides`. */
  anchor: Point;
  /** The sides to try, the preferred one first. */
  sides: Side[];
  gap: number;
  /** Whether to try the corners of a side before its middle, as for a node beside its wires. */
  corner?: boolean;
  w: number;
  h: number;
}

export interface Placement extends Rect {
  /** Whether the label had to move away from its anchor, so a line should lead to it. */
  displaced: boolean;
}

/** Space kept free around every label. */
const MARGIN = 2;
/** How far each ring of candidates lies beyond the one before. */
const STEP = GRID / 2;

const overlaps = (p: Rect, q: Rect) =>
  p.x - MARGIN < q.x + q.w && q.x < p.x + p.w + MARGIN && p.y - MARGIN < q.y + q.h && q.y < p.y + p.h + MARGIN;

/** The box of a label on `side` at distance `d`, moved `slide` along that side. */
function box(label: LabelRequest, side: Side, d: number, slide: number): Rect {
  const { anchor: a, w, h } = label;
  switch (side) {
    case 'below':
      return { x: a.x - w / 2 + slide, y: a.y + d, w, h };
    case 'above':
      return { x: a.x - w / 2 + slide, y: a.y - d - h, w, h };
    case 'right':
      return { x: a.x + d, y: a.y - h / 2 + slide, w, h };
    case 'left':
      return { x: a.x - d - w, y: a.y - h / 2 + slide, w, h };
  }
}

/** The slides along a side: centred, a quarter off, and past the anchor into either corner. */
function slides(label: LabelRequest, side: Side): number[] {
  const length = side === 'below' || side === 'above' ? label.w : label.h;
  const corner = length / 2 + label.gap;
  return label.corner
    ? [corner, -corner, 0, length / 4, -length / 4]
    : [0, length / 4, -length / 4, corner, -corner];
}

function bounds(rects: Rect[]): { x0: number; y0: number; x1: number; y1: number } {
  let x0 = Infinity;
  let y0 = Infinity;
  let x1 = -Infinity;
  let y1 = -Infinity;
  for (const r of rects) {
    x0 = Math.min(x0, r.x);
    y0 = Math.min(y0, r.y);
    x1 = Math.max(x1, r.x + r.w);
    y1 = Math.max(y1, r.y + r.h);
  }
  return { x0, y0, x1, y1 };
}

/**
 * Greedy, in the order given: each label takes the first free candidate, trying ring after ring
 * outwards and on each ring every side and slide. Beyond the bounding box of everything placed so
 * far every candidate is free, so the search always ends, at the latest just outside that box.
 */
export function placeLabels(labels: LabelRequest[], obstacles: Rect[]): Map<string, Placement> {
  const taken: Rect[] = [...obstacles];
  const placed = new Map<string, Placement>();
  for (const label of labels) {
    const free = (r: Rect) => !taken.some((t) => overlaps(r, t));
    const all = bounds([...taken, { x: label.anchor.x, y: label.anchor.y, w: 0, h: 0 }]);
    // The distance at which a box on any side has left the bounding box completely.
    const reach =
      Math.max(label.anchor.x - all.x0, all.x1 - label.anchor.x, label.anchor.y - all.y0, all.y1 - label.anchor.y) +
      MARGIN +
      1;
    let found: Placement | null = null;
    for (let ring = 0; !found; ring += 1) {
      const d = label.gap + ring * STEP;
      if (d > reach) {
        found = { ...box(label, label.sides[0], Math.max(d, reach), 0), displaced: true };
        break;
      }
      for (const side of label.sides) {
        for (const slide of slides(label, side)) {
          const candidate = box(label, side, d, slide);
          if (free(candidate)) {
            found = { ...candidate, displaced: ring > 0 };
            break;
          }
        }
        if (found) break;
      }
    }
    taken.push(found);
    placed.set(label.id, found);
  }
  return placed;
}

/** The box around points turned with a part and moved to its place. */
function turned(element: EditorElement, corners: Point[]): Rect {
  const points = corners.map((p) => rotate(p, element.rotation)).map((p) => ({ x: element.x + p.x, y: element.y + p.y }));
  const b = bounds(points.map((p) => ({ ...p, w: 0, h: 0 })));
  return { x: b.x0, y: b.y0, w: b.x1 - b.x0, h: b.y1 - b.y0 };
}

/** Half the size of a part's body across its axis and along it, as `Symbol` draws it. */
const BODY: Partial<Record<EditorElement['kind'], { along: number; across: number }>> = {
  resistor: { along: 20, across: 7 },
  potentiometer: { along: 20, across: 7 },
  voltageSource: { along: 14, across: 14 },
  currentSource: { along: 14, across: 14 },
  powerLoad: { along: 14, across: 14 },
};

/** A line as a thin box around it, as wide as the stroke and the terminal circles. */
function segment(a: Point, b: Point, half: number): Rect {
  return {
    x: Math.min(a.x, b.x) - half,
    y: Math.min(a.y, b.y) - half,
    w: Math.abs(b.x - a.x) + 2 * half,
    h: Math.abs(b.y - a.y) + 2 * half,
  };
}

/** The drawn body of a part: the symbol without its leads, a wire's line, a ground's bars. */
export function body(element: EditorElement): Rect {
  if (element.kind === 'wire') {
    const { a, b } = terminals(element) as Record<'a' | 'b', Point>;
    return segment(a, b, 2);
  }
  if (element.kind === 'ground') return turned(element, [{ x: -12, y: 0 }, { x: 12, y: 19 }]);
  const { along, across } = BODY[element.kind]!;
  return turned(element, [{ x: -along, y: -across }, { x: along, y: across }]);
}

/** What the labels must keep clear of: the bodies and leads of the parts, the wires, the terminals. */
export function obstacles(elements: EditorElement[]): Rect[] {
  const rects: Rect[] = [];
  for (const element of elements) {
    const places = terminals(element);
    for (const point of Object.values(places) as Point[]) rects.push(segment(point, point, 5));
    rects.push(body(element));
    if (element.kind === 'wire' || element.kind === 'ground') continue;
    rects.push(turned(element, [{ x: -HALF, y: -2 }, { x: HALF, y: 2 }]));
    if (places.wiper) rects.push(turned(element, [{ x: -4, y: -HALF }, { x: 4, y: 0 }]));
  }
  return rects;
}

/** The point on a box nearest to `p`, where a line from `p` to the box ends. */
export function nearest(r: Rect, p: Point): Point {
  return { x: Math.min(Math.max(p.x, r.x), r.x + r.w), y: Math.min(Math.max(p.y, r.y), r.y + r.h) };
}
