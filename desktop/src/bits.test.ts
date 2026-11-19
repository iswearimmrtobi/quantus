import { describe, expect, it } from 'vitest';
import { lowBits, patternInput, uprightHex } from './bits';

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

describe('base forms', () => {
  it('strips the upright marks and groups binary digits from the right', async () => {
    const { plainDigits, groupedBinary } = await import('./components/BaseForms');
    expect(plainDigits('\\mathrm{FF}')).toBe('FF');
    expect(plainDigits('-5')).toBe('-5');
    expect(groupedBinary('101101')).toBe('10 1101');
    expect(groupedBinary('1111')).toBe('1111');
  });
});

describe('lowBits', () => {
  it('keeps the low bits of a flipped value when the word shrinks', () => {
    const cut = lowBits('0000000110000001', 8);
    expect(cut).toBe('10000001');
    expect(patternInput(cut, true, 'dec')).toBe('-127');
    expect(patternInput(cut, false, 'hex')).toBe('81');
  });
});
