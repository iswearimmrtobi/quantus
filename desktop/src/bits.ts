import type { BitView, NumberBase } from './api';

/** The pattern with bit `index` (0 = least significant) flipped. */
export function flipBit(view: BitView, index: number): string {
  const at = view.bits.length - 1 - index;
  const flipped = view.bits[at] === '1' ? '0' : '1';
  return view.bits.slice(0, at) + flipped + view.bits.slice(at + 1);
}

/** The number a pattern stands for: two's complement when signed. */
export function patternValue(bits: string, signed: boolean): bigint {
  const unsigned = BigInt(`0b${bits}`);
  return signed && bits[0] === '1' ? unsigned - (1n << BigInt(bits.length)) : unsigned;
}

const RADIX: Record<NumberBase, number> = { dec: 10, hex: 16, bin: 2, oct: 8 };

/**
 * The pattern as Basis-N input in `base`: the number in decimal (negative when signed and the
 * top bit is set), the pattern without leading zeros in the others, which the word reads back
 * as the same bits.
 */
export function patternInput(bits: string, signed: boolean, base: NumberBase): string {
  if (base === 'dec') return patternValue(bits, signed).toString();
  const digits = BigInt(`0b${bits}`).toString(RADIX[base]).toUpperCase();
  return /[A-F]/.test(digits) ? `\\mathrm{${digits}}` : digits;
}
