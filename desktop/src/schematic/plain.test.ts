import { describe, expect, it } from 'vitest';
import { plainOf } from './plain';

describe('plainOf', () => {
  it('reads numbers with units', () => {
    expect(plainOf('33{,}75\\,\\mathrm{V}')).toBe('33,75 V');
    expect(plainOf('2.5\\,\\mathrm{mA}')).toBe('2.5 mA');
    expect(plainOf('1\\,\\mathrm{k\\Omega}')).toBe('1 kΩ');
    expect(plainOf('470\\,\\Omega')).toBe('470 Ω');
  });

  it('reads powers of ten and fractions', () => {
    expect(plainOf('4{,}7\\cdot 10^{-3}\\,\\mathrm{A}')).toBe('4,7·10⁻³ A');
    expect(plainOf('\\frac{1}{3}\\,\\mathrm{A}')).toBe('1/3 A');
    expect(plainOf('-\\frac{10}{3}\\,\\mathrm{V}')).toBe('-10/3 V');
    expect(plainOf('5\\,\\mathrm{m}^{2}')).toBe('5 m²');
  });

  it('reads a percentage', () => {
    expect(plainOf('40\\,\\%')).toBe('40 %');
  });
});
