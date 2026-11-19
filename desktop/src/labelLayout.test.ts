import { describe, expect, it } from 'vitest';
import { nearest, placeLabels, type LabelRequest } from './labelLayout';

const label = (id: string, x: number, y: number, rest: Partial<LabelRequest> = {}): LabelRequest => ({
  id,
  anchor: { x, y },
  sides: ['below', 'above', 'right', 'left'],
  gap: 10,
  w: 20,
  h: 10,
  ...rest,
});

/** Something far off: without any obstacle every spot counts as beyond the drawing, so displaced. */
const far = { x: 500, y: 500, w: 1, h: 1 };

describe('placeLabels', () => {
  it('puts a free label on its first side', () => {
    const placed = placeLabels([label('a', 0, 0)], [far]);
    expect(placed.get('a')).toEqual({ x: -10, y: 10, w: 20, h: 10, displaced: false, offset: { x: -10, y: 10 } });
  });

  it('takes the next side when the first is blocked', () => {
    const placed = placeLabels([label('a', 0, 0)], [far, { x: -30, y: 5, w: 60, h: 30 }]);
    expect(placed.get('a')).toMatchObject({ x: -10, y: -20, displaced: false });
  });

  it('keeps labels apart and moves one out when all near spots are taken', () => {
    const blocked = [{ x: -100, y: -100, w: 200, h: 95 }, { x: -100, y: 5, w: 200, h: 95 }, { x: -100, y: -5, w: 85, h: 10 }, { x: 15, y: -5, w: 85, h: 10 }];
    const placed = placeLabels([label('a', 0, 0), label('b', 0, 0)], blocked);
    expect(placed.get('a')!.displaced).toBe(true);
    expect(placed.get('b')!.displaced).toBe(true);
    const [a, b] = [placed.get('a')!, placed.get('b')!];
    const apart = a.x + a.w <= b.x || b.x + b.w <= a.x || a.y + a.h <= b.y || b.y + b.h <= a.y;
    expect(apart).toBe(true);
  });

  it('tries the corners first for a corner label', () => {
    const placed = placeLabels([label('n', 0, 0, { corner: true, sides: ['above'] })], [far]);
    expect(placed.get('n')).toMatchObject({ x: 10, y: -20 });
  });

  it('keeps the previous spot while it is free, moving with the anchor', () => {
    const before = placeLabels([label('a', 0, 0)], [far, { x: -30, y: 5, w: 60, h: 30 }]);
    const after = placeLabels([label('a', 40, 0)], [], before);
    expect(after.get('a')).toMatchObject({ x: 30, y: -20 });
  });
});

describe('nearest', () => {
  it('ends a leader on the box', () => {
    expect(nearest({ x: 0, y: 0, w: 10, h: 10 }, { x: 20, y: 5 })).toEqual({ x: 10, y: 5 });
    expect(nearest({ x: 0, y: 0, w: 10, h: 10 }, { x: 5, y: 5 })).toEqual({ x: 5, y: 5 });
  });
});
