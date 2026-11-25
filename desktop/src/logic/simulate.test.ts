import { describe, expect, it } from 'vitest';
import type { GateType, LogicElement } from '../api';
import { rowInputs, simulate } from './simulate';

/** f = A ∧ B from two NANDs. */
const elements: LogicElement[] = [
  { id: 'A', kind: 'input', name: 'A', output: 0 },
  { id: 'B', kind: 'input', name: 'B', output: 1 },
  { id: 'N1', kind: 'gate', gate: 'nand', inputs: [0, 1], output: 2 },
  { id: 'N2', kind: 'gate', gate: 'nand', inputs: [2, 2], output: 3 },
  { id: 'f', kind: 'output', name: 'f', inputs: [3] },
];

describe('simulate', () => {
  it('reads row 3 as A = 1, B = 1', () => {
    expect([...rowInputs(['A', 'B'], 2)]).toEqual([
      ['A', true],
      ['B', false],
    ]);
  });

  it('works the nets out row by row', () => {
    const on = simulate({ elements }, rowInputs(['A', 'B'], 3));
    expect([on.get(2), on.get(3)]).toEqual([false, true]);
    const off = simulate({ elements }, rowInputs(['A', 'B'], 1));
    expect([off.get(2), off.get(3)]).toEqual([true, false]);
  });

  it('leaves a loop undecided', () => {
    const loop: LogicElement[] = [{ id: 'N', kind: 'gate', gate: 'not', inputs: [5], output: 5 }];
    expect(simulate({ elements: loop }, new Map()).get(5)).toBeUndefined();
  });
});

/** One gate with `n` inputs A, B, … on nets 1 to n, its output on net 100. */
function oneGate(gate: GateType, n: number): LogicElement[] {
  const names = Array.from({ length: n }, (_, k) => String.fromCharCode(65 + k));
  return [
    ...names.map((name, k): LogicElement => ({ id: name, kind: 'input', name, output: k + 1 })),
    { id: 'G1', kind: 'gate', gate, inputs: names.map((_, k) => k + 1), output: 100 },
  ];
}

/** The rows (the first input the MSB) where the gate is 1. */
function onesOf(gate: GateType, n: number): number[] {
  const names = Array.from({ length: n }, (_, k) => String.fromCharCode(65 + k));
  const ones: number[] = [];
  for (let row = 0; row < 1 << n; row++) {
    if (simulate({ elements: oneGate(gate, n) }, rowInputs(names, row)).get(100)) ones.push(row);
  }
  return ones;
}

/** The rows of `n` inputs where `f` of the input values is true. */
function onesBy(n: number, f: (v: boolean[]) => boolean): number[] {
  const ones: number[] = [];
  for (let row = 0; row < 1 << n; row++) {
    const v = Array.from({ length: n }, (_, k) => ((row >> (n - 1 - k)) & 1) === 1);
    if (f(v)) ones.push(row);
  }
  return ones;
}

const odd = (v: boolean[]) => v.filter(Boolean).length % 2 === 1;

describe('every gate type', () => {
  // The same tables as the engine's (crates/quantus-core/tests/logic_operators.rs).
  it('has the textbook table for two inputs', () => {
    expect(onesOf('and', 2)).toEqual([3]);
    expect(onesOf('or', 2)).toEqual([1, 2, 3]);
    expect(onesOf('xor', 2)).toEqual([1, 2]);
    expect(onesOf('xnor', 2)).toEqual([0, 3]);
    expect(onesOf('nand', 2)).toEqual([0, 1, 2]);
    expect(onesOf('nor', 2)).toEqual([0]);
  });

  it('inverts or passes one input', () => {
    expect(onesOf('not', 1)).toEqual([0]);
    expect(onesOf('buffer', 1)).toEqual([1]);
  });

  it('works for every input count up to eight, XOR and XNOR as odd and even parity', () => {
    const reference: [GateType, (v: boolean[]) => boolean][] = [
      ['and', (v) => v.every(Boolean)],
      ['or', (v) => v.some(Boolean)],
      ['nand', (v) => !v.every(Boolean)],
      ['nor', (v) => !v.some(Boolean)],
      ['xor', odd],
      ['xnor', (v) => !odd(v)],
    ];
    for (const [gate, f] of reference) {
      for (let n = 2; n <= 8; n++) expect(onesOf(gate, n), `${gate} of ${n}`).toEqual(onesBy(n, f));
    }
  });

  it('leaves its output undecided while an input is open', () => {
    const open: LogicElement[] = [
      { id: 'A', kind: 'input', name: 'A', output: 1 },
      { id: 'G1', kind: 'gate', gate: 'and', inputs: [1, null], output: 100 },
    ];
    expect(simulate({ elements: open }, rowInputs(['A'], 1)).get(100)).toBeUndefined();
  });
});
