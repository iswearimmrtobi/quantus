import { describe, expect, it } from 'vitest';
import { allPins, localBody, newPart, numberNets, partPins, toNetlist, withGate, type LogicPart } from './logicCircuit';

const wire = (id: string, x: number, y: number, x2: number, y2: number): LogicPart => ({ id, kind: 'wire', x, y, x2, y2, rotation: 0 });

/** Two switches A and B into an AND gate whose output lights the lamp f, joined by wires. */
function andCircuit(): LogicPart[] {
  const a = newPart([], 'input', { x: 0, y: 0 });
  const b = newPart([a], 'input', { x: 0, y: 40 });
  const gate = newPart([a, b], 'and', { x: 100, y: 20 });
  const lamp = newPart([a, b, gate], 'output', { x: 200, y: 20 });
  const [inA, inB] = partPins(gate).inputs;
  return [
    a,
    b,
    gate,
    lamp,
    wire('W1', 20, 0, inA.x, 0),
    wire('W2', inA.x, 0, inA.x, inA.y),
    wire('W3', 20, 40, inB.x, 40),
    wire('W4', inB.x, 40, inB.x, inB.y),
    wire('W5', partPins(gate).output!.x, 20, partPins(lamp).inputs[0].x, 20),
  ];
}

describe('parts', () => {
  it('names new switches and lamps freely', () => {
    const parts = andCircuit();
    expect(parts.slice(0, 4).map((p) => [p.id, p.name ?? p.gate])).toEqual([
      ['I1', 'A'],
      ['I2', 'B'],
      ['G1', 'and'],
      ['O1', 'f'],
    ]);
    expect(newPart(parts, 'input', { x: 0, y: 0 }).name).toBe('C');
    expect(newPart([], 'not', { x: 0, y: 0 })).toMatchObject({ kind: 'gate', gate: 'not', inputs: 1 });
    expect(newPart([], 'wire', { x: 0, y: 0 })).toMatchObject({ x2: 80, y2: 0 });
  });

  it('places the pins of gates, switches and lamps, turned with them', () => {
    const gate = newPart([], 'and', { x: 0, y: 0 });
    expect(partPins(gate)).toEqual({ inputs: [{ x: -40, y: -20 }, { x: -40, y: 20 }], output: { x: 40, y: 0 } });
    const out = partPins({ ...gate, rotation: 1 }).output!;
    expect([out.x + 0, out.y]).toEqual([0, 40]);
    expect(allPins(newPart([], 'input', { x: 0, y: 0 }))).toEqual([{ x: 20, y: 0 }]);
    expect(allPins(newPart([], 'output', { x: 0, y: 0 }))).toEqual([{ x: -20, y: 0 }]);
    expect(allPins(withGate({ ...gate, inputs: 3 }, 'or'))).toHaveLength(4);
  });

  it('boxes the bodies', () => {
    expect(localBody(newPart([], 'nand', { x: 0, y: 0 }))).toEqual([-20, -30, 28, 30]);
    expect(localBody(newPart([], 'not', { x: 0, y: 0 }))).toEqual([-20, -16, 28, 16]);
  });
});

describe('nets', () => {
  it('joins pins through wires and numbers the nets from 1', () => {
    const parts = andCircuit();
    const nets = numberNets(parts);
    const [inA, inB] = partPins(parts[2]).inputs;
    const net = (x: number, y: number) => nets.byPoint.get(`${x},${y}`);
    expect(net(20, 0)).toBe(net(inA.x, inA.y));
    expect(net(20, 40)).toBe(net(inB.x, inB.y));
    expect(net(20, 0)).not.toBe(net(20, 40));
    expect(new Set(nets.byPoint.values())).toEqual(new Set([1, 2, 3]));
    // The switch's pin, the gate's input and both ends of W1 and W2.
    expect(nets.size.get(net(20, 0)!)).toBe(6);
  });

  it('joins a pin in the middle of a wire', () => {
    const lamp2 = newPart(andCircuit(), 'output', { x: 120, y: 60 });
    // A wire from the gate's output down past the second lamp's pin at (100, 60).
    const parts = [...andCircuit(), lamp2, wire('W6', 140, 20, 140, 60), wire('W7', 140, 60, 80, 60)];
    const nets = numberNets(parts);
    expect(nets.byPoint.get('100,60')).toBe(nets.byPoint.get('140,20'));
  });

  it('makes the netlist, a lone pin not connected', () => {
    const parts = andCircuit().filter((p) => p.id !== 'W5');
    expect(toNetlist(parts)).toEqual({
      elements: [
        { id: 'I1', kind: 'input', name: 'A', output: 1 },
        { id: 'I2', kind: 'input', name: 'B', output: 2 },
        { id: 'G1', kind: 'gate', gate: 'and', inputs: [1, 2], output: null },
        { id: 'O1', kind: 'output', name: 'f', inputs: [null] },
      ],
    });
    expect(toNetlist([])).toBeNull();
  });
});
