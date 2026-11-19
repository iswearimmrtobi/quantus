import type { BitView, NumberBase } from './api';

/** The pattern with bit `index` (0 = least significant) flipped. */
export function flipBit(view: BitView, index: number): string {
  const at = view.bits.length - 1 - index;
  const flipped = view.bits[at] === '1' ? '0' : '1';
  return view.bits.slice(0, at) + flipped + view.bits.slice(at + 1);
}

/** The lowest `width` bits of a pattern, as a register of that width keeps them. */
export function lowBits(bits: string, width: number): string {
  return bits.slice(-width);
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

/** A run of hex digits, some of them maybe already set upright by an earlier pass. */
const HEX_RUN = /(?:\\mathrm\{[0-9A-F]+\}|[0-9A-F])+/g;

/** The argument of `f32(…)`/`f64(…)` is a decimal float (`1E3`), not hex: its range in `latex`. */
function floatArguments(latex: string): [number, number][] {
  const ranges: [number, number][] = [];
  const name = /(?:\\operatorname\{|\\mathrm\{)?f(?:32|64)\}?/g;
  for (let match = name.exec(latex); match; match = name.exec(latex)) {
    let at = match.index + match[0].length;
    if (latex.startsWith('\\left(', at)) at += '\\left('.length;
    else if (latex[at] === '(') at += 1;
    else continue;
    const start = at;
    let depth = 1;
    while (at < latex.length && depth > 0) {
      if (latex[at] === '(') depth++;
      else if (latex[at] === ')') depth--;
      at++;
    }
    ranges.push([start, at]);
  }
  return ranges;
}

/**
 * Hex numbers in typed Basis-N LaTeX set upright: MathLive writes a typed `FF` as two italic
 * variables, the engine writes it `\mathrm{FF}` (as [patternInput] does). A run of digits with
 * at least one of the letters A–F becomes one `\mathrm{…}`; lowercase letters are the prefixes
 * and words (`h`, `and`) and stay, and so does the decimal argument of `f32(…)`.
 */
export function uprightHex(latex: string): string {
  const floats = floatArguments(latex);
  return latex.replace(HEX_RUN, (run, offset: number) => {
    if (floats.some(([start, end]) => offset >= start && offset < end)) return run;
    // A command name such as `\mathrm` is lowercase, so the run cannot sit inside one; but a run
    // right after a backslash would be a command's argument-less tail and is left alone.
    if (latex[offset - 1] === '\\') return run;
    const digits = run.replace(/\\mathrm\{([0-9A-F]+)\}/g, '$1');
    if (!/[A-F]/.test(digits)) return run;
    const upright = `\\mathrm{${digits}}`;
    return upright === run ? run : upright;
  });
}
