import { describe, expect, it } from 'vitest';
import type { LogicElement } from '../api';
import { gateByGate } from './byGate';

/** f = A ∧ B as NAND: G1 = NAND(A, B), G2 = NAND(G1, G1). */
const elements: LogicElement[] = [
  { id: 'A', kind: 'input', name: 'A', output: 0 },
  { id: 'B', kind: 'input', name: 'B', output: 1 },
  { id: 'N2', kind: 'gate', gate: 'nand', inputs: [2, 2], output: 3 },
  { id: 'N1', kind: 'gate', gate: 'nand', inputs: [0, 1], output: 2 },
  { id: 'f', kind: 'output', name: 'f', inputs: [3] },
];

describe('gateByGate', () => {
  it('names the gates from the inputs on and ends with the lamp', () => {
    expect(gateByGate({ elements }, 'logic')).toBe(
      '\\begin{aligned} G_{1} &= \\overline{A \\land B} \\\\ G_{2} &= \\overline{G_{1} \\land G_{1}} \\\\ f &= G_{2} \\end{aligned}',
    );
  });

  it('writes the technical notation', () => {
    expect(gateByGate({ elements }, 'technical')).toContain('\\overline{A \\cdot B}');
  });

  it('writes a lamp fed by an input straight', () => {
    const wire: LogicElement[] = [
      { id: 'A', kind: 'input', name: 'A', output: 0 },
      { id: 'g', kind: 'output', name: 'g', inputs: [0] },
    ];
    expect(gateByGate({ elements: wire }, 'logic')).toBe('\\begin{aligned} g &= A \\end{aligned}');
  });
});
