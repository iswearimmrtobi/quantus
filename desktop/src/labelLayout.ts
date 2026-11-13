import { boundsOf, GRID, type Point, type Rect } from './schematic';

export type { Rect };

/**
 * Places the labels on a schematic's canvas so that none overlaps another label or an obstacle (a
 * part, a wire). Everything is in drawing coordinates, so zooming and moving the canvas leave the
 * layout alone. What the obstacles are is up to the editor (`obstacles` in `circuit.ts`).
 */

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
  /** The top left corner relative to the anchor, so the spot can be kept when the anchor moves. */
  offset: Point;
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
  const b = boundsOf(rects);
  return { x0: b.x, y0: b.y, x1: b.x + b.w, y1: b.y + b.h };
}

/**
 * Greedy, in the order given: each label takes the first free candidate, trying ring after ring
 * outwards and on each ring every side and slide. Beyond the bounding box of everything placed so
 * far every candidate is free, so the search always ends, at the latest just outside that box.
 *
 * With `previous`, a label whose size has not changed first tries the spot it had there, at the
 * same offset from its anchor, so it moves along with its part and stays put while others move.
 * Those that keep their spot are placed before the rest, which then search around them.
 */
export function placeLabels(
  labels: LabelRequest[],
  obstacles: Rect[],
  previous?: Map<string, Placement>,
): Map<string, Placement> {
  const taken: Rect[] = [...obstacles];
  const placed = new Map<string, Placement>();
  const free = (r: Rect) => !taken.some((t) => overlaps(r, t));
  const take = (label: LabelRequest, r: Rect, displaced: boolean) => {
    const placement = { ...r, displaced, offset: { x: r.x - label.anchor.x, y: r.y - label.anchor.y } };
    taken.push(placement);
    placed.set(label.id, placement);
  };

  for (const label of labels) {
    const before = previous?.get(label.id);
    if (!before || before.w !== label.w || before.h !== label.h) continue;
    const spot = { x: label.anchor.x + before.offset.x, y: label.anchor.y + before.offset.y, w: label.w, h: label.h };
    if (free(spot)) take(label, spot, before.displaced);
  }

  for (const label of labels) {
    if (placed.has(label.id)) continue;
    const all = bounds([...taken, { x: label.anchor.x, y: label.anchor.y, w: 0, h: 0 }]);
    // The distance at which a box on any side has left the bounding box completely.
    const reach =
      Math.max(label.anchor.x - all.x0, all.x1 - label.anchor.x, label.anchor.y - all.y0, all.y1 - label.anchor.y) +
      MARGIN +
      1;
    let found: { box: Rect; displaced: boolean } | null = null;
    for (let ring = 0; !found; ring += 1) {
      const d = label.gap + ring * STEP;
      if (d > reach) {
        found = { box: box(label, label.sides[0], d, 0), displaced: true };
        break;
      }
      for (const side of label.sides) {
        for (const slide of slides(label, side)) {
          const candidate = box(label, side, d, slide);
          if (free(candidate)) {
            found = { box: candidate, displaced: ring > 0 };
            break;
          }
        }
        if (found) break;
      }
    }
    take(label, found.box, found.displaced);
  }
  return placed;
}

/** The point on a box nearest to `p`, where a line from `p` to the box ends. */
export function nearest(r: Rect, p: Point): Point {
  return { x: Math.min(Math.max(p.x, r.x), r.x + r.w), y: Math.min(Math.max(p.y, r.y), r.y + r.h) };
}
