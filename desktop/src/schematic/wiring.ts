import { onPath, pinAcrossAt, pointKey, route, simplify, wireEnds, wirePoints, withPath, type Path, type Placeable, type Point } from './geometry';

/**
 * Wires that keep up with the parts: an end on a pin goes where the pin goes, the wire laid anew
 * across and down; and two wires that meet end to end with nothing else there become one.
 */

/** A wire end that sits on a pin of a part that moves, and the wire as it was before. */
export interface Attachment<E> {
  wire: E;
  end: 'a' | 'b';
  part: string;
  /** Which of the part's pins, by its place in `pins(part)`. */
  pin: number;
}

/**
 * The ends of the wires not in `moving` that sit on a pin of a part in `moving`, a moving wire's
 * end included: those follow when the parts move or turn.
 */
export function attachments<E extends Placeable>(elements: E[], moving: Set<string>, pins: (e: E) => Point[]): Attachment<E>[] {
  const pinAt = new Map<string, { part: string; pin: number }>();
  for (const element of elements) {
    if (!moving.has(element.id)) continue;
    pins(element).forEach((point, pin) => {
      if (!pinAt.has(pointKey(point))) pinAt.set(pointKey(point), { part: element.id, pin });
    });
  }
  const found: Attachment<E>[] = [];
  for (const wire of elements) {
    if (wire.kind !== 'wire' || moving.has(wire.id)) continue;
    const [a, b] = wireEnds(wire);
    for (const [end, point] of [['a', a], ['b', b]] as const) {
      const at = pinAt.get(pointKey(point));
      if (at) found.push({ wire, end, ...at });
    }
  }
  return found;
}

/**
 * Which pins of the parts in `moving` touch which wires not in `moving`, at an end, a corner or in
 * the middle, as `part:pin|wire`. A part is connected by drawing wires, never by being moved onto
 * one: see `touchesNewWire`.
 */
export function wireContacts<E extends Placeable>(elements: E[], moving: Set<string>, pins: (e: E) => Point[]): Set<string> {
  const wires = elements.filter((e) => e.kind === 'wire' && !moving.has(e.id)).map((wire) => ({ id: wire.id, path: wirePoints(wire) }));
  const contacts = new Set<string>();
  for (const part of elements) {
    if (part.kind === 'wire' || !moving.has(part.id)) continue;
    pins(part).forEach((point, pin) => {
      const key = pointKey(point);
      for (const { id, path } of wires) {
        if (key === pointKey(path[0]) || key === pointKey(path[path.length - 1]) || onPath(path, point)) contacts.add(`${part.id}:${pin}|${id}`);
      }
    });
  }
  return contacts;
}

/** Whether moving or turning the parts of `moving` from `before` to `after` puts a pin on a wire it did not touch. */
export function touchesNewWire<E extends Placeable>(before: E[], after: E[], moving: Set<string>, pins: (e: E) => Point[]): boolean {
  const had = wireContacts(before, moving, pins);
  return [...wireContacts(after, moving, pins)].some((contact) => !had.has(contact));
}

/** Whether a path leaves its start sideways: a straight path that goes nowhere counts as sideways. */
const leavesAcross = (path: Path) => path[0].y === path[1].y;

/** Which way a wire leaves its ends `a` and `b`, by the pins there: sideways (true), upright, or unknown (null). */
export interface EndsAcross {
  a?: boolean | null;
  b?: boolean | null;
}

/**
 * `wire` with its ends moved to `a` and `b`. Moved alike, it moves as a whole. A wire of up to two
 * corners is laid anew with `route`, leaving each end the way its pin points (`across`), or else
 * the way it left it before; a longer one keeps its corners and only its last leg at each moved end
 * slides along.
 */
export function rerouted<E extends Placeable>(wire: E, a: Point, b: Point, across: EndsAcross = {}): E {
  const path = wirePoints(wire);
  const [oldA, oldB] = [path[0], path[path.length - 1]];
  const [da, db] = [{ x: a.x - oldA.x, y: a.y - oldA.y }, { x: b.x - oldB.x, y: b.y - oldB.y }];
  if (da.x === db.x && da.y === db.y) return withPath(wire, path.map((p) => ({ x: p.x + da.x, y: p.y + da.y })));
  if (path.length <= 4) {
    // Led from the end that stays, so that it keeps its way out of its pin.
    const aStays = da.x === 0 && da.y === 0;
    const bStays = db.x === 0 && db.y === 0;
    const atA = across.a ?? null;
    const atB = across.b ?? null;
    if (bStays && !aStays) {
      const leaves = atB ?? leavesAcross([...path].reverse());
      return withPath(wire, route(b, a, leaves, atA ?? leaves).reverse());
    }
    const leaves = atA ?? leavesAcross(path);
    return withPath(wire, route(a, b, leaves, atB ?? leaves));
  }
  return withPath(wire, slid(slid(path, b).reverse(), a).reverse());
}

/**
 * The way of `wire` with its `end` pulled to `at`. A straight wire stays straight: the end moves
 * along the axis it is pulled most. One with corners is laid anew from its other end.
 */
export function pulledEnd<E extends Placeable>(wire: E, end: 'a' | 'b', at: Point): Path {
  const path = wirePoints(wire);
  const [a, b] = [path[0], path[path.length - 1]];
  if (path.length === 2) {
    const other = end === 'a' ? b : a;
    const pulled = Math.abs(at.x - other.x) >= Math.abs(at.y - other.y) ? { x: at.x, y: other.y } : { x: other.x, y: at.y };
    return end === 'a' ? [pulled, other] : [other, pulled];
  }
  return wirePoints(end === 'a' ? rerouted(wire, at, b) : rerouted(wire, a, at));
}

/**
 * `path` with its last point moved to `to`: the corner before the last leg slides with it, so the
 * last leg keeps its direction and the one before it stretches.
 */
function slid(path: Path, to: Point): Path {
  const n = path.length;
  const [before, corner] = [path[n - 3], path[n - 2]];
  const along = before.y === corner.y;
  const moved = along ? { x: to.x, y: before.y } : { x: before.x, y: to.y };
  return [...path.slice(0, n - 2), moved, to];
}

/** `elements` with each attached wire end where its pin is now, the wire laid anew. */
export function followed<E extends Placeable>(elements: E[], attached: Attachment<E>[], pins: (e: E) => Point[]): E[] {
  if (!attached.length) return elements;
  const byId = new Map(elements.map((e) => [e.id, e]));
  const ends = new Map<string, { wire: E; a?: Point; b?: Point }>();
  for (const { wire, end, part, pin } of attached) {
    const moved = byId.get(part);
    const point = moved && pins(moved)[pin];
    if (!point) continue;
    const entry = ends.get(wire.id) ?? { wire };
    entry[end] = point;
    ends.set(wire.id, entry);
  }
  return elements.map((element) => {
    const entry = ends.get(element.id);
    if (!entry) return element;
    const [oldA, oldB] = wireEnds(entry.wire);
    const [a, b] = [entry.a ?? oldA, entry.b ?? oldB];
    // Each end leaves the way the pin it sits on points: an upright pin and a sideways one make an L.
    const across = { a: pinAcrossAt(elements, a, pins), b: pinAcrossAt(elements, b, pins) };
    // Its whole way anew: a wire laid straight again must not keep the corners it had a step before.
    return withPath(element, wirePoints(rerouted(entry.wire, a, b, across)));
  });
}

/** The fields that make a wire what it is, besides where it runs. */
const PLACE = new Set(['id', 'kind', 'x', 'y', 'x2', 'y2', 'bends', 'rotation']);

/** Whether two wires are alike but for where they run: the same material, the same resistance. */
function alike(p: Placeable, q: Placeable): boolean {
  const fields = (e: Placeable) =>
    Object.entries(e)
      .filter(([key, value]) => !PLACE.has(key) && value !== undefined && value !== '')
      .sort(([k], [l]) => k.localeCompare(l));
  return JSON.stringify(fields(p)) === JSON.stringify(fields(q));
}

/** A drawing after `mergedWires`, and the id of the wire each merged one went into. */
export interface Merged<E> {
  elements: E[];
  into: Map<string, string>;
}

/**
 * `elements` with every two wires that meet end to end merged into one: where exactly two pins or
 * wire ends sit, both of them ends of `mergeable` wires alike, and no wire of `joining` passes
 * through. The wire drawn first takes the other in and keeps its id. `elements` itself comes back
 * when nothing was merged.
 */
export function mergedWires<E extends Placeable>(
  elements: E[],
  pins: (e: E) => Point[],
  joining: (elements: E[]) => Path[],
  mergeable: (wire: E) => boolean,
): Merged<E> {
  let current = elements;
  const into = new Map<string, string>();
  for (;;) {
    const merge = nextMerge(current, pins, joining(current), mergeable);
    if (!merge) return { elements: current, into };
    const [first, second] = merge;
    const lead = wirePoints(first.wire);
    const tail = wirePoints(second.wire);
    // From the first wire's other end to where they meet, on to the second wire's other end.
    const toMeeting = first.end === 'b' ? lead : [...lead].reverse();
    const fromMeeting = second.end === 'a' ? tail : [...tail].reverse();
    const joined = withPath(first.wire, simplify([...toMeeting, ...fromMeeting.slice(1)]));
    current = current.filter((e) => e.id !== second.wire.id).map((e) => (e.id === first.wire.id ? joined : e));
    into.set(second.wire.id, first.wire.id);
    for (const [gone, kept] of into) if (kept === second.wire.id) into.set(gone, first.wire.id);
  }
}

/** The first point where two wires meet end to end and may be merged, with the two in drawing order. */
function nextMerge<E extends Placeable>(elements: E[], pins: (e: E) => Point[], paths: Path[], mergeable: (wire: E) => boolean) {
  const at = new Map<string, { element: E; end: 'a' | 'b' | null }[]>();
  for (const element of elements) {
    const points = pins(element);
    const [a, b] = element.kind === 'wire' ? wireEnds(element) : [null, null];
    points.forEach((point) => {
      const key = pointKey(point);
      const end = a && pointKey(a) === key ? 'a' : b && pointKey(b) === key ? 'b' : null;
      at.set(key, [...(at.get(key) ?? []), { element, end }]);
    });
  }
  for (const [key, meeting] of at) {
    if (meeting.length !== 2) continue;
    const [p, q] = meeting;
    if (p.element === q.element || !p.end || !q.end || p.element.kind !== 'wire' || q.element.kind !== 'wire') continue;
    if (!mergeable(p.element) || !mergeable(q.element) || !alike(p.element, q.element)) continue;
    const point = key.split(',').map(Number);
    const here = { x: point[0], y: point[1] };
    if (paths.some((path) => onPath(path, here))) continue;
    // Two wires that close a ring stay two: one wire cannot start and end at the same point.
    const [pOther, qOther] = [p.end === 'a' ? wireEnds(p.element)[1] : wireEnds(p.element)[0], q.end === 'a' ? wireEnds(q.element)[1] : wireEnds(q.element)[0]];
    if (pointKey(pOther) === pointKey(qOther)) continue;
    const order = elements.indexOf(p.element) < elements.indexOf(q.element);
    const [first, second] = order ? [p, q] : [q, p];
    return [
      { wire: first.element, end: first.end },
      { wire: second.element, end: second.end },
    ] as const;
  }
  return null;
}
