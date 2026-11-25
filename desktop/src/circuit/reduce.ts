import type { MergeStep } from '../api';
import { GRID, HALF, inside, isIdealWire, pointKey, snap, straightPieces, terminals, type EditorElement, type Point } from './model';

export type { MergeStep };

/**
 * The drawings of the Kirchhoff mode's reduction: the network after each merge of resistors, laid
 * out on the grid from the drawing the user made.
 */

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
export function reducedDrawings(drawing: EditorElement[], merges: MergeStep[]): { elements: EditorElement[]; merged: string[] }[] {
  // The preview lays out straight wires only.
  const elements = straightPieces(drawing);
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
