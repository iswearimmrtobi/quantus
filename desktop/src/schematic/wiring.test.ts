import { describe, expect, it } from 'vitest';
import { ends, idealPaths, isIdealWire, toKirchhoffRequest, type EditorElement } from '../circuit/model';
import { pasteInto } from './clipboard';
import { drawnPath, longestPiece, movedBy, onPath, rotatedQuarter, route, simplify, wirePoints, withPath, type Point } from './geometry';
import { joinPoints } from './nets';
import { attachments, followed, mergedWires, pulledEnd, rerouted } from './wiring';

const p = (x: number, y: number): Point => ({ x, y });
const resistor = (id: string, x: number, y: number, rotation = 0): EditorElement => ({ id, kind: 'resistor', x, y, rotation });
const wire = (id: string, ...points: Point[]): EditorElement => withPath({ id, kind: 'wire', x: 0, y: 0, rotation: 0 }, points);
const merged = (elements: EditorElement[]) => mergedWires(elements, ends, idealPaths, isIdealWire);

describe('route', () => {
  it('lays a wire between two pins at different heights with equal sideways stubs', () => {
    expect(route(p(40, 0), p(200, 80), true)).toEqual([p(40, 0), p(120, 0), p(120, 80), p(200, 80)]);
  });
  it('leaves downwards with equal upright stubs when not across', () => {
    expect(route(p(0, 40), p(80, 200), false)).toEqual([p(0, 40), p(0, 120), p(80, 120), p(80, 200)]);
  });
  it('keeps the corner on the grid when the middle is not', () => {
    const path = route(p(0, 0), p(60, 40), true);
    expect(path).toEqual([p(0, 0), p(40, 0), p(40, 40), p(60, 40)]);
  });
  it('runs straight between points in line', () => {
    expect(route(p(0, 0), p(80, 0), false)).toEqual([p(0, 0), p(80, 0)]);
  });
  it('draws into the open as an L, between pins as a Z', () => {
    expect(drawnPath(p(0, 0), p(80, 40), true, false)).toEqual([p(0, 0), p(80, 0), p(80, 40)]);
    expect(drawnPath(p(0, 0), p(80, 40), true, true)).toEqual([p(0, 0), p(40, 0), p(40, 40), p(80, 40)]);
  });
});

describe('simplify', () => {
  it('drops repeated points and points in the middle of a straight run', () => {
    expect(simplify([p(0, 0), p(20, 0), p(20, 0), p(60, 0), p(60, 40)])).toEqual([p(0, 0), p(60, 0), p(60, 40)]);
  });
  it('keeps both ends of a wire that goes nowhere', () => {
    expect(simplify([p(20, 20), p(20, 20)])).toEqual([p(20, 20), p(20, 20)]);
  });
});

describe('onPath', () => {
  it('counts a corner and the middle of a piece, not the ends', () => {
    const path = [p(0, 0), p(40, 0), p(40, 40)];
    expect(onPath(path, p(40, 0))).toBe(true);
    expect(onPath(path, p(40, 20))).toBe(true);
    expect(onPath(path, p(0, 0))).toBe(false);
    expect(onPath(path, p(40, 40))).toBe(false);
    expect(onPath(path, p(20, 20))).toBe(false);
  });
  it('joins a point on the second piece to the wire start', () => {
    const root = joinPoints([p(0, 0), p(40, 20)], [], [[p(0, 0), p(40, 0), p(40, 40)]]);
    expect(root('40,20')).toBe(root('0,0'));
  });
});

describe('bends move and turn with their wire', () => {
  it('moves the corners', () => {
    const w = wire('W1', p(0, 0), p(40, 0), p(40, 40));
    const [moved] = movedBy([w], new Map([[w.id, w]]), 20, 20);
    expect(wirePoints(moved)).toEqual([p(20, 20), p(60, 20), p(60, 60)]);
  });
  it('turns the corners', () => {
    const w = wire('W1', p(0, 0), p(40, 0), p(40, 40));
    const [turned] = rotatedQuarter([w], ['W1'], ends);
    expect(wirePoints(turned)).toEqual([p(0, 0), p(0, 40), p(-40, 40)]);
  });
});

describe('wires follow their parts', () => {
  // R1's pins at (-40, 0) and (40, 0); R2's at (160, 0) and (240, 0).
  const r1 = resistor('R1', 0, 0);
  const r2 = resistor('R2', 200, 0);
  const w1 = wire('W1', p(40, 0), p(160, 0));

  it('lays the wire anew with equal stubs when one part moves up', () => {
    const elements = [r1, r2, w1];
    const attached = attachments(elements, new Set(['R2']), ends);
    expect(attached).toEqual([{ wire: w1, end: 'b', part: 'R2', pin: 0 }]);
    const moved = followed(movedBy(elements, new Map([['R2', r2]]), 0, 80), attached, ends);
    expect(wirePoints(moved[2])).toEqual([p(40, 0), p(100, 0), p(100, 80), p(160, 80)]);
  });

  it('lays the wire straight again when the part comes back in line during the drag', () => {
    const elements = [r1, r2, w1];
    const attached = attachments(elements, new Set(['R2']), ends);
    const origins = new Map([['R2', r2]]);
    // Each step of a drag starts from the drawing of the step before, the wire bent by then.
    const down = followed(movedBy(elements, origins, 0, 80), attached, ends);
    const level = followed(movedBy(down, origins, 40, 0), attached, ends);
    expect(wirePoints(level[2])).toEqual([p(40, 0), p(200, 0)]);
    expect(level[2]).not.toHaveProperty('bends');
  });

  it('moves a wire whose two ends both move alike as a whole', () => {
    const w = wire('W2', p(40, 0), p(40, 40), p(-40, 40), p(-40, 0));
    const elements = [r1, w];
    const attached = attachments(elements, new Set(['R1']), ends);
    const moved = followed(movedBy(elements, new Map([['R1', r1]]), 20, 0), attached, ends);
    expect(wirePoints(moved[1])).toEqual([p(60, 0), p(60, 40), p(-20, 40), p(-20, 0)]);
  });

  it('follows a turned part', () => {
    const elements = [r1, w1];
    const attached = attachments(elements, new Set(['R1']), ends);
    const turned = followed(rotatedQuarter(elements, ['R1'], ends), attached, ends);
    // R1's b pin is now at (0, 40).
    expect(wirePoints(turned[1])[0]).toEqual(p(0, 40));
    expect(wirePoints(turned[1]).at(-1)).toEqual(p(160, 0));
  });

  it('slides only the last leg of a wire with many corners', () => {
    const w = wire('W1', p(0, 0), p(40, 0), p(40, 40), p(80, 40), p(80, 80), p(120, 80));
    expect(wirePoints(rerouted(w, p(0, 0), p(120, 120)))).toEqual([p(0, 0), p(40, 0), p(40, 40), p(80, 40), p(80, 120), p(120, 120)]);
  });
});

describe('pulling a wire end', () => {
  it('keeps a straight wire straight, along the axis pulled most', () => {
    expect(pulledEnd(wire('W1', p(0, 0), p(40, 0)), 'b', p(100, 20))).toEqual([p(0, 0), p(100, 0)]);
    expect(pulledEnd(wire('W1', p(0, 0), p(40, 0)), 'a', p(20, 80))).toEqual([p(40, 80), p(40, 0)]);
  });
  it('lays a wire with corners anew from its other end', () => {
    expect(pulledEnd(wire('W1', p(0, 0), p(40, 0), p(40, 40)), 'b', p(80, 40))).toEqual([p(0, 0), p(40, 0), p(40, 40), p(80, 40)]);
  });
});

describe('copied wires', () => {
  it('are pasted with their corners', () => {
    const w = wire('W1', p(0, 0), p(40, 0), p(40, 40));
    const [pasted] = pasteInto([w], [w], 40, (part) => ({ ...part, id: 'W2' }));
    expect(wirePoints(pasted)).toEqual([p(40, 40), p(80, 40), p(80, 80)]);
  });
});

describe('longestPiece', () => {
  it('is where a wire with corners has its label', () => {
    expect(longestPiece(wire('W1', p(0, 0), p(20, 0), p(20, 80), p(40, 80)))).toEqual([p(20, 0), p(20, 80)]);
  });
});

describe('mergedWires', () => {
  it('merges two wires in line into one', () => {
    const result = merged([wire('W1', p(0, 0), p(40, 0)), wire('W2', p(40, 0), p(100, 0))]);
    expect(result.elements).toHaveLength(1);
    expect(wirePoints(result.elements[0])).toEqual([p(0, 0), p(100, 0)]);
    expect(result.into.get('W2')).toBe('W1');
  });

  it('merges two wires around a corner, and a chain of three', () => {
    const result = merged([wire('W1', p(0, 0), p(40, 0)), wire('W3', p(40, 40), p(80, 40)), wire('W2', p(40, 40), p(40, 0))]);
    expect(result.elements.map((e) => e.id)).toEqual(['W1']);
    expect(wirePoints(result.elements[0])).toEqual([p(0, 0), p(40, 0), p(40, 40), p(80, 40)]);
    expect(result.into).toEqual(new Map([['W3', 'W1'], ['W2', 'W1']]));
  });

  it('leaves wires that meet at a pin, at a third wire, or on another wire', () => {
    const atPin = [resistor('R1', 0, 0), wire('W1', p(40, 0), p(80, 0)), wire('W2', p(40, 0), p(40, 40))];
    expect(merged(atPin).elements).toBe(atPin);
    const three = [wire('W1', p(0, 0), p(40, 0)), wire('W2', p(40, 0), p(80, 0)), wire('W3', p(40, 0), p(40, 40))];
    expect(merged(three).elements).toBe(three);
    const onWire = [wire('W1', p(0, 0), p(40, 0)), wire('W2', p(40, 0), p(40, 40)), wire('W3', p(40, -40), p(40, 0), p(80, 0))];
    expect(merged(onWire).elements.map((e) => e.id)).toEqual(['W1', 'W2', 'W3']);
  });

  it('leaves a wire with a resistance, and wires that close a ring', () => {
    const resistive = [wire('W1', p(0, 0), p(40, 0)), { ...wire('W2', p(40, 0), p(80, 0)), rOverride: '1\\Omega' }];
    expect(merged(resistive).elements).toBe(resistive);
    const ring = [wire('W1', p(0, 0), p(40, 0), p(40, 40)), wire('W2', p(40, 40), p(0, 40), p(0, 0))];
    expect(merged(ring).elements).toBe(ring);
  });
});

describe('bent wires for the Kirchhoff engine', () => {
  it('goes as straight pieces, the first with the id and values', () => {
    const w = { ...wire('W1', p(0, 0), p(40, 0), p(40, 40)), material: 'copper' } as EditorElement;
    const request = toKirchhoffRequest([w]);
    expect(request.elements).toEqual([
      expect.objectContaining({ id: 'W1', material: 'copper', pa: { x: 0, y: 0 }, pb: { x: 2, y: 0 } }),
      { id: 'W1#b1', kind: 'wire', pa: { x: 2, y: 0 }, pb: { x: 2, y: 2 } },
    ]);
    expect(request.elements[0]).not.toHaveProperty('bends');
  });
});
