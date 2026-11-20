import { describe, expect, it } from 'vitest';
import { asDefinition, assignNames, typedName } from './functionNames';

describe('function names', () => {
  it('reads the typed name', () => {
    expect(typedName('g(x) = x^2')).toBe('g');
    expect(typedName('f_{1}\\left(t\\right)=t')).toBe('f_{1}');
    expect(typedName('x^2')).toBeNull();
    expect(typedName('y = x')).toBeNull();
  });

  it('gives the others the next free letters', () => {
    expect(assignNames(['x^2', 'g(x) = 1', '2x', '3x'])).toEqual(['f', 'g', 'h', 'k']);
    expect(assignNames(['f(x)=1', 'x'])).toEqual(['f', 'g']);
  });

  it('writes a definition with its name', () => {
    expect(asDefinition('x^2', 'g')).toBe('g(x) = x^2');
    expect(asDefinition('y = 2x', 'h')).toBe('h(x) =  2x');
    expect(asDefinition('k(t) = t', 'f')).toBe('k(t) = t');
  });
});
