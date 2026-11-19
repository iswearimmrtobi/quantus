import { describe, expect, it } from 'vitest';
import {
  boundsOf,
  connectedPointsOf,
  corner,
  freshIdOf,
  GRID,
  inRectangle,
  inside,
  junctionDotsOf,
  movedBy,
  rotate,
  rotatedQuarter,
  segmentBox,
  snap,
  wireEnds,
  type Placeable,
  type Point,
} from './geometry';

const p = (x: number, y: number): Point => ({ x, y });
const part = (id: string, x: number, y: number, rotation = 0): Placeable => ({ id, kind: 'part', x, y, rotation });
const wire = (id: string, a: Point, b: Point): Placeable => ({ id, kind: 'wire', x: a.x, y: a.y, rotation: 0, x2: b.x, y2: b.y });
/** A part's pins: one grid step left and right of it, turned with it. */
const pins = (e: Placeable) =>
  e.kind === 'wire' ? wireEnds(e) : [p(-GRID, 0), p(GRID, 0)].map((q) => rotate(q, e.rotation)).map((q) => p(e.x + q.x, e.y + q.y));

describe('grid and points', () => {
  it('snaps to the grid', () => {
    expect([snap(9), snap(11), snap(-11), snap(30)]).toEqual([0, 20, -20, 40]);
  });

  it('turns by quarters, either way', () => {
    expect(rotate(p(1, 2), 1)).toEqual(p(-2, 1));
    expect(rotate(p(1, 2), 2)).toEqual(p(-1, -2));
    expect(rotate(p(1, 2), 3)).toEqual(p(2, -1));
    expect(rotate(p(1, 2), -1)).toEqual(p(2, -1));
    expect(rotate(p(1, 2), 4)).toEqual(p(1, 2));
  });

  it('finds points inside a segment, not on its ends', () => {
    expect(inside(p(0, 0), p(40, 0), p(20, 0))).toBe(true);
    expect(inside(p(0, 0), p(40, 0), p(40, 0))).toBe(false);
    expect(inside(p(0, 0), p(40, 0), p(60, 0))).toBe(false);
    expect(inside(p(0, 0), p(0, 40), p(0, 20))).toBe(true);
    expect(inside(p(0, 0), p(40, 0), p(20, 20))).toBe(false);
  });

  it('bends a slanted wire into an L', () => {
    expect(corner(p(0, 0), p(40, 20), true)).toEqual(p(40, 0));
    expect(corner(p(0, 0), p(40, 20), null)).toEqual(p(40, 0));
    expect(corner(p(0, 0), p(40, 20), false)).toEqual(p(0, 20));
  });

  it('boxes segments and bounds boxes', () => {
    expect(segmentBox(p(40, 0), p(0, 0), 2)).toEqual({ x: -2, y: -2, w: 44, h: 4 });
    expect(boundsOf([{ x: 0, y: 0, w: 10, h: 10 }, { x: -5, y: 20, w: 1, h: 1 }])).toEqual({ x: -5, y: 0, w: 15, h: 21 });
  });

  it('hands out the next free id', () => {
    expect(freshIdOf(['R1', 'R2', 'R4'], 'R')).toBe('R3');
    expect(freshIdOf([], 'W')).toBe('W1');
  });
});

describe('connectivity', () => {
  it('marks points where two ends meet or an end sits on a joining wire', () => {
    const ends = [p(0, 0), p(40, 0), p(40, 0), p(80, 0), p(20, 40)];
    const joining: [Point, Point][] = [[p(0, 40), p(40, 40)]];
    expect([...connectedPointsOf(ends, joining)].sort()).toEqual(['20,40', '40,0']);
  });

  it('puts junction dots where three or more lines meet, a middle counting twice', () => {
    const ends = [p(0, 0), p(40, 0), p(40, 0), p(40, 0), p(20, 40), p(80, 0), p(80, 0)];
    const joining: [Point, Point][] = [[p(0, 40), p(40, 40)]];
    expect(junctionDotsOf(ends, joining)).toEqual([p(40, 0), p(20, 40)]);
  });
});

describe('moving, turning, choosing', () => {
  const elements = [part('R1', 0, 0), wire('W1', p(20, 0), p(60, 0)), part('R2', 100, 100, 1)];

  it('moves parts and both ends of wires from their origins', () => {
    const origins = new Map(elements.slice(0, 2).map((e) => [e.id, e]));
    const moved = movedBy(elements, origins, 20, -20);
    expect(moved[0]).toMatchObject({ x: 20, y: -20 });
    expect(moved[1]).toMatchObject({ x: 40, y: -20, x2: 80, y2: -20 });
    expect(moved[2]).toBe(elements[2]);
  });

  it('turns one part about itself', () => {
    const turned = rotatedQuarter(elements, ['R2'], pins);
    expect(turned[2]).toMatchObject({ x: 100, y: 100, rotation: 2 });
    expect(rotatedQuarter(elements, ['R2'], pins, (r) => r + 0.5)[2]).toMatchObject({ rotation: 1.5 });
  });

  it('turns a wire about its start', () => {
    expect(rotatedQuarter(elements, ['W1'], pins)[1]).toMatchObject({ x: 20, y: 0, x2: 20, y2: 40 });
  });

  it('turns several parts about the middle of their pins', () => {
    const turned = rotatedQuarter(elements, ['R1', 'W1'], pins);
    // Pins from -20 to 60 across, 0 down: the middle is (20, 0).
    expect(turned[0]).toMatchObject({ x: 20, y: -20, rotation: 1 });
    expect(turned[1]).toMatchObject({ x: 20, y: 0, x2: 20, y2: 40 });
  });

  it('chooses the parts with their place or a pin in a rectangle', () => {
    expect(inRectangle(elements, p(15, -5), p(25, 5), pins)).toEqual(['R1', 'W1']);
    expect(inRectangle(elements, p(90, 70), p(110, 90), pins)).toEqual(['R2']);
    expect(inRectangle(elements, p(200, 200), p(300, 300), pins)).toEqual([]);
  });
});
