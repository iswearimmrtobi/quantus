import { describe, expect, it } from 'vitest';
import { LABEL_GAP, spreadLabels } from './geometryLabels';

describe('spreadLabels', () => {
  it('puts a lone label above and right of its anchor', () => {
    const placed = spreadLabels([{ id: 'A', x: 100, y: 100, text: 'A' }], []);
    const a = placed.get('A')!;
    expect(a.x).toBe(100 + LABEL_GAP);
    expect(a.y + a.h).toBe(100 - LABEL_GAP);
  });

  it('moves two labels at the same point apart', () => {
    const placed = spreadLabels(
      [
        { id: 'A', x: 100, y: 100, text: 'A' },
        { id: 'K', x: 101, y: 100, text: 'K' },
      ],
      [],
    );
    const [a, k] = [placed.get('A')!, placed.get('K')!];
    const overlap = a.x < k.x + k.w && k.x < a.x + a.w && a.y < k.y + k.h && k.y < a.y + a.h;
    expect(overlap).toBe(false);
  });

  it('keeps a label off another point', () => {
    // g's anchor right under B's marker: above-right of g would cover B.
    const placed = spreadLabels([{ id: 'g', x: 100, y: 120, text: 'g' }], [{ x: 110, y: 104 }]);
    const g = placed.get('g')!;
    const covers = g.x < 114 && 106 < g.x + g.w && g.y < 108 && 100 < g.y + g.h;
    expect(covers).toBe(false);
  });
});
