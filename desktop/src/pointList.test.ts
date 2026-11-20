import { describe, expect, it } from 'vitest';
import { isPointList } from './pointList';

describe('isPointList', () => {
  it('takes lists of two or more points', () => {
    expect(isPointList('(0,1),(1,3),(2,2)')).toBe(true);
    expect(isPointList('\\left(0|1\\right);\\ \\left(2|5\\right)')).toBe(true);
    expect(isPointList('(0 \\mid 1), (1 \\mid 0)')).toBe(true);
    expect(isPointList('(0;\\frac{1}{2}),(1;2)')).toBe(true);
  });

  it('leaves functions and single points alone', () => {
    expect(isPointList('(x+1)(x-1)')).toBe(false);
    expect(isPointList('(1,2)')).toBe(false);
    expect(isPointList('x^2')).toBe(false);
    expect(isPointList('(x+1)^2')).toBe(false);
  });
});
