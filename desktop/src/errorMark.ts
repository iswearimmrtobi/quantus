import type { MathfieldElement } from 'mathlive';
import type { Span } from './api';

/**
 * Marking the part of the input an error is about.
 *
 * The backend counts a span in characters (code points) of the LaTeX it was sent; a text field
 * counts UTF-16 units, and MathLive counts positions in its own model, so both are translated
 * here.
 */

/** The text `span` covers in `latex`. */
export function spanText(latex: string, span: Span): string {
  return Array.from(latex).slice(span.start, span.end).join('');
}

/** `span` in UTF-16 units of `latex`, as `setSelectionRange` counts. */
export function utf16Range(latex: string, span: Span): [number, number] {
  const chars = Array.from(latex);
  const start = chars.slice(0, span.start).join('').length;
  return [start, start + spanText(latex, span).length];
}

/** LaTeX without what MathLive may write differently for a part than for the whole. */
const compact = (latex: string) => latex.replace(/\\left|\\right|\s+/g, '');

/**
 * The range of MathLive's model whose LaTeX is `span` of `latex` (the field's value). Where
 * that text occurs more than once, the occurrence after the most similar amount of input wins.
 */
export function modelRange(mathfield: MathfieldElement, latex: string, span: Span): [number, number] | null {
  const target = compact(spanText(latex, span));
  if (!target) return null;
  const before = compact(Array.from(latex).slice(0, span.start).join('')).length;
  const last = mathfield.lastOffset;
  let best: { range: [number, number]; distance: number } | null = null;
  for (let start = 0; start < last; start++) {
    for (let end = start + 1; end <= last; end++) {
      const text = compact(mathfield.getValue(start, end, 'latex'));
      if (text === target) {
        const distance = Math.abs(compact(mathfield.getValue(0, start, 'latex')).length - before);
        if (!best || distance < best.distance) best = { range: [start, end], distance };
        break;
      }
      // The part only grows from here on.
      if (text.length > 2 * target.length + 16) break;
    }
  }
  return best?.range ?? null;
}
