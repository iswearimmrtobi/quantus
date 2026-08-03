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
