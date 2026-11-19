import { describe, expect, it } from 'vitest';
import { uprightHex } from './bits';

describe('uprightHex', () => {
  it('sets a typed hex number upright', () => {
    expect(uprightHex('FF')).toBe('\\mathrm{FF}');
    expect(uprightHex('1F\\cdot3+1')).toBe('\\mathrm{1F}\\cdot3+1');
  });

  it('joins a digit typed after an upright number', () => {
    expect(uprightHex('\\mathrm{FF}F')).toBe('\\mathrm{FFF}');
    expect(uprightHex('\\mathrm{1F}0')).toBe('\\mathrm{1F0}');
  });

  it('leaves decimal numbers, words and prefixes alone', () => {
    expect(uprightHex('123+4')).toBe('123+4');
    expect(uprightHex('\\mathrm{FF}')).toBe('\\mathrm{FF}');
    expect(uprightHex('a \\operatorname{and} b')).toBe('a \\operatorname{and} b');
    expect(uprightHex('\\mathrm{h}12')).toBe('\\mathrm{h}12');
  });

  it('leaves the float argument of f32 alone', () => {
    expect(uprightHex('\\operatorname{f32}\\left(1E3\\right)+A')).toBe('\\operatorname{f32}\\left(1E3\\right)+\\mathrm{A}');
  });
});
