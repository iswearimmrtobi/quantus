import { onPath, pointKey, type Path, type Point } from './geometry';

/**
 * Which points of a drawing are joined: pins and wire ends on the same grid point are one point
 * already; `through` wires join their two ends, and every wire of `middles` joins whatever sits
 * in its middle or on a corner to its start. Returns the root of each point's group, by `pointKey`; two points
 * are joined when their roots are the same. Points not in `points` are their own root.
 *
 * Logic's wires join both their ends and their middles (`through` and `middles` alike). The
 * electronics mode's ideal wires join only their middles: a wire is an element of the solver
 * there, its two ends two nodes.
 */
export function joinPoints(points: Iterable<Point>, through: Path[], middles: Path[]): (key: string) => string {
  const parent = new Map<string, string>();
  const root = (key: string): string => {
    let k = key;
    while (parent.has(k)) k = parent.get(k)!;
    // Shorten the path for the next look-up.
    for (let j = key; j !== k; ) {
      const next = parent.get(j)!;
      parent.set(j, k);
      j = next;
    }
    return k;
  };
  const join = (p: string, q: string) => {
    const [a, b] = [root(p), root(q)];
    if (a !== b) parent.set(a, b);
  };
  const unique = new Map<string, Point>();
  for (const point of points) unique.set(pointKey(point), point);
  for (const path of through) join(pointKey(path[0]), pointKey(path[path.length - 1]));
  for (const path of middles) {
    for (const [key, point] of unique) if (onPath(path, point)) join(key, pointKey(path[0]));
  }
  return root;
}
