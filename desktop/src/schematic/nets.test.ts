import { describe, expect, it } from 'vitest';
import type { Point, Segment } from './geometry';
import { joinPoints } from './nets';

const p = (x: number, y: number): Point => ({ x, y });

describe('joinPoints', () => {
  const points = [p(0, 0), p(40, 0), p(20, 0), p(20, 40), p(80, 0)];

  it('leaves every point alone without wires', () => {
    const root = joinPoints(points, [], []);
    expect(new Set(points.map((q) => root(`${q.x},${q.y}`))).size).toBe(5);
  });

  it('joins the ends of through wires', () => {
    const root = joinPoints(points, [[p(0, 0), p(40, 0)]], []);
    expect(root('0,0')).toBe(root('40,0'));
    expect(root('20,0')).not.toBe(root('0,0'));
  });

  it('joins a point in the middle to the start only', () => {
    const root = joinPoints(points, [], [[p(0, 0), p(40, 0)]]);
    expect(root('20,0')).toBe(root('0,0'));
    expect(root('40,0')).not.toBe(root('0,0'));
  });

  it('joins a point in the middle of two wires to both', () => {
    const wires: Segment[] = [
      [p(0, 0), p(40, 0)],
      [p(20, 40), p(20, -40)],
    ];
    const root = joinPoints([...points, p(20, -40)], [], wires);
    expect(root('0,0')).toBe(root('20,40'));
  });

  it('follows chains of wires', () => {
    const root = joinPoints(points, [[p(0, 0), p(40, 0)], [p(40, 0), p(80, 0)]], []);
    expect(root('0,0')).toBe(root('80,0'));
    expect(root('9,9')).toBe('9,9');
  });
});
