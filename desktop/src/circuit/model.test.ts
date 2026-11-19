import { describe, expect, it } from 'vitest';
import {
  body,
  connectedPoints,
  freshId,
  fromFile,
  junctionDots,
  junctionPoint,
  newElement,
  numberNodes,
  obstacles,
  terminals,
  toFile,
  toRequest,
  type EditorElement,
} from './model';

const at = (kind: EditorElement['kind'], id: string, x: number, y: number, rotation = 0, rest: Partial<EditorElement> = {}): EditorElement => ({
  id,
  kind,
  x,
  y,
  rotation,
  ...rest,
});
const wire = (id: string, x: number, y: number, x2: number, y2: number, rest: Partial<EditorElement> = {}): EditorElement => ({
  id,
  kind: 'wire',
  x,
  y,
  x2,
  y2,
  rotation: 0,
  ...rest,
});

/**
 * A source and two resistors: V1 standing at (0, 80) from (0, 40) to (0, 120), R1 across the top
 * from (40, 0) to (120, 0), R2 down the right from (160, 40) to (160, 120), joined by ideal wires
 * and a ground under the source.
 */
const loop: EditorElement[] = [
  at('voltageSource', 'V1', 0, 80, 1, { u: '10V' }),
  at('resistor', 'R1', 80, 0, 0, { r: '1kOhm' }),
  at('resistor', 'R2', 160, 80, 1, { r: '2kOhm' }),
  wire('W1', 0, 40, 0, 0),
  wire('W2', 0, 0, 40, 0),
  wire('W3', 120, 0, 160, 0),
  wire('W4', 160, 0, 160, 40),
  wire('W5', 0, 120, 160, 120),
  at('ground', 'GND1', 0, 120),
];

describe('terminals', () => {
  it('places a part’s terminals two grid steps out, turned with it', () => {
    expect(terminals(loop[1])).toEqual({ a: { x: 40, y: 0 }, b: { x: 120, y: 0 } });
    expect(terminals(loop[0])).toEqual({ a: { x: 0, y: 40 }, b: { x: 0, y: 120 } });
  });

  it('keeps a resistor at 45° on the grid', () => {
    expect(terminals(at('resistor', 'R', 0, 0, 0.5))).toEqual({ a: { x: -40, y: -40 }, b: { x: 40, y: 40 } });
    expect(terminals(at('resistor', 'R', 0, 0, 1.5))).toEqual({ a: { x: 40, y: -40 }, b: { x: -40, y: 40 } });
  });

  it('gives a potentiometer its wiper', () => {
    const wiper = terminals(at('potentiometer', 'P', 0, 0, 2)).wiper!;
    expect([wiper.x + 0, wiper.y]).toEqual([0, 40]);
  });

  it('gives a wire its two ends and a ground one point', () => {
    expect(terminals(loop[3])).toEqual({ a: { x: 0, y: 40 }, b: { x: 0, y: 0 } });
    expect(terminals(loop[8])).toEqual({ ground: { x: 0, y: 120 } });
  });
});

describe('nodes', () => {
  it('numbers the nodes, ground 0, each wire end its own', () => {
    const nodes = numberNodes(loop);
    expect(nodes.grounded).toBe(true);
    expect(nodes.byPoint.get('0,120')).toBe(0);
    expect(nodes.byPoint.get('160,120')).toBe(nodes.byPoint.get('160,120'));
    // Ideal wires are elements of the solver, so their two ends are different nodes.
    expect(nodes.byPoint.get('0,40')).not.toBe(nodes.byPoint.get('0,0'));
    expect(new Set(nodes.byPoint.values()).size).toBe(nodes.pointOf.size);
    expect(toRequest(loop, nodes).nodes).toEqual([...nodes.pointOf.keys()].sort((p, q) => p - q));
  });

  it('grounds the negative terminal of the first source without a ground symbol', () => {
    const nodes = numberNodes(loop.filter((e) => e.kind !== 'ground'));
    expect(nodes.grounded).toBe(false);
    expect(nodes.byPoint.get('0,120')).toBe(0);
  });

  it('joins a terminal in the middle of an ideal wire to the wire’s start', () => {
    // R3 hangs from the middle of W5.
    const nodes = numberNodes([...loop, at('resistor', 'R3', 80, 160, 1)]);
    expect(nodes.byPoint.get('80,120')).toBe(nodes.byPoint.get('0,120'));
  });

  it('joins nothing in the middle of a resistive wire', () => {
    const resistive = loop.map((e) => (e.id === 'W5' ? { ...e, rOverride: '1Ohm' } : e));
    const nodes = numberNodes([...resistive, at('resistor', 'R3', 80, 160, 1)]);
    expect(nodes.byPoint.get('80,120')).not.toBe(nodes.byPoint.get('0,120'));
  });

  it('turns the drawing into a request', () => {
    const request = toRequest(loop, numberNodes(loop));
    expect(request.elements.map((e) => e.id)).toEqual(['V1', 'R1', 'R2', 'W1', 'W2', 'W3', 'W4', 'W5']);
    expect(request.elements[0]).toMatchObject({ kind: 'voltageSource', u: '10V' });
    expect(request.elements[0].b).toBe(0);
  });

  it('keeps a file round trip', () => {
    expect(fromFile(toFile(loop))).toEqual(loop);
  });
});

describe('dots and boxes', () => {
  it('marks connected terminals and junctions', () => {
    const withR3 = [...loop, at('resistor', 'R3', 80, 160, 1)];
    const connected = connectedPoints(withR3);
    expect(connected.has('80,120')).toBe(true);
    expect(connected.has('0,40')).toBe(true);
    expect(connected.has('80,200')).toBe(false);
    // The ground, the source and W5 meet under the source; R3 meets W5 in its middle.
    expect(junctionDots(withR3)).toEqual([{ x: 0, y: 120 }, { x: 80, y: 120 }]);
  });

  it('draws no dots for the terminals A and B', () => {
    const marks = [wire('W1', 0, 0, 40, 0), wire('W2', 40, 0, 80, 0), at('terminal', 'A', 40, 0)];
    expect(junctionDots(marks)).toEqual([]);
  });

  it('boxes bodies and obstacles', () => {
    expect(body(loop[1])).toEqual({ x: 60, y: -7, w: 40, h: 14 });
    // At 45° the box holds the turned body's four corners: (20 + 7) / √2 out on every side.
    expect(body(at('resistor', 'R', 0, 0, 0.5)).w).toBeCloseTo((2 * 27) / Math.SQRT2, 6);
    expect(obstacles(loop)).toMatchSnapshot();
    expect(obstacles([at('potentiometer', 'P', 0, 0), at('resistor', 'R', 200, 0, 0.5), at('terminal', 'A', 0, 100)])).toMatchSnapshot();
  });
});

describe('new parts', () => {
  it('numbers the ids by kind and names the terminals A and B', () => {
    expect(freshId(loop, 'resistor')).toBe('R3');
    expect(freshId(loop, 'wire')).toBe('W6');
    expect(freshId([], 'terminal')).toBe('A');
    expect(freshId([at('terminal', 'A', 0, 0)], 'terminal')).toBe('B');
    expect(freshId([at('terminal', 'A', 0, 0), at('terminal', 'B', 0, 0)], 'terminal')).toBe('T1');
  });

  it('snaps a new part and gives a new wire its length', () => {
    expect(newElement(loop, 'resistor', { x: 11, y: 29 })).toEqual({ id: 'R3', kind: 'resistor', x: 20, y: 20, rotation: 0 });
    expect(newElement([], 'wire', { x: 0, y: 0 })).toEqual({ id: 'W1', kind: 'wire', x: 0, y: 0, x2: 80, y2: 0, rotation: 0 });
  });
});

describe('junctions', () => {
  it('joins a terminal on two crossing ideal wires to both', () => {
    // W6 crosses W5 at (80, 120); R3 hangs from there.
    const nodes = numberNodes([...loop, wire('W6', 80, 80, 80, 140), at('resistor', 'R3', 80, 160, 1)]);
    expect(nodes.byPoint.get('80,120')).toBe(nodes.byPoint.get('0,120'));
    expect(nodes.byPoint.get('80,80')).toBe(nodes.byPoint.get('0,120'));
  });

  it('puts a junction’s dot where the most lines meet', () => {
    const withR3 = [...loop, at('resistor', 'R3', 80, 160, 1)];
    expect(junctionPoint(withR3, { x: 0, y: 120 })).toEqual({ x: 80, y: 120 });
    expect(junctionPoint(withR3, { x: 999, y: 999 })).toEqual({ x: 999, y: 999 });
  });
});
