import { describe, expect, it } from 'vitest';
import type { LogicElement } from '../api';
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
