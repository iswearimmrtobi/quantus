import { useMemo } from 'react';
import katex from 'katex';

interface Props {
  latex: string;
  display?: boolean;
  className?: string;
}

/**
 * Static LaTeX: steps, results, variables, history.
 *
 * `throwOnError` stays off on purpose — a half-typed formula should show as much as it can
 * instead of tearing a hole in the view.
 */
export function Katex({ latex, display = false, className }: Props) {
  const html = useMemo(
    () => katex.renderToString(latex, { throwOnError: false, displayMode: display, output: 'html' }),
    [latex, display],
  );
  return <span className={className} dangerouslySetInnerHTML={{ __html: html }} />;
}

/** Commands whose two arguments sit above and below each other. */
const STACKED = new Set(['frac', 'dfrac', 'tfrac', 'cfrac', 'binom', 'dbinom', 'tbinom']);
/** Operators that reach above and below the line in display style. */
const TALL = new Set(['lim', 'limsup', 'liminf', 'int', 'iint', 'iiint', 'oint', 'sum', 'prod', 'coprod', 'bigcup', 'bigcap']);

/**
 * How many lines a formula stands tall: 1 for flat text, 2 for a fraction, limit or
 * integral, 3 for a fraction inside a fraction, and so on.
 *
 * Counted from the source rather than measured, so the preview row has its height before
 * KaTeX has laid anything out and does not jump while typing.
 */
export function latexLines(latex: string): number {
  // One frame per open brace group: its depth, and how many arguments of a fraction
  // written directly in it are still to come (they sit one deeper).
  type Frame = { depth: number; pending: number };
  const frames: Frame[] = [{ depth: 0, pending: 0 }];
  let deepest = 0;
  for (let i = 0; i < latex.length; i++) {
    const c = latex[i];
    const frame = frames[frames.length - 1];
    if (c === '\\') {
      const name = /^[a-zA-Z]+/.exec(latex.slice(i + 1))?.[0];
      if (!name) {
        i++;
        continue;
      }
      i += name.length;
      if (frame.pending > 0) {
        // `\frac\pi2`: a command standing in for a whole argument.
        frame.pending--;
        deepest = Math.max(deepest, frame.depth + 1);
      }
      if (STACKED.has(name)) {
        frame.pending = 2;
      } else if (TALL.has(name)) {
        deepest = Math.max(deepest, frame.depth + 1);
      }
    } else if (c === '{') {
      let depth = frame.depth;
      if (frame.pending > 0) {
        frame.pending--;
        depth++;
        deepest = Math.max(deepest, depth);
      }
      frames.push({ depth, pending: 0 });
    } else if (c === '}') {
      if (frames.length > 1) frames.pop();
    } else if (frame.pending > 0 && !/\s/.test(c)) {
      // `\frac12`: single characters as arguments.
      frame.pending--;
      deepest = Math.max(deepest, frame.depth + 1);
    }
  }
  return 1 + deepest;
}
