import { useEffect, useState, type MouseEvent } from 'react';
import { api, type LibraryHit } from '../api';
import { useT } from '../i18n';
import { Katex } from './Katex';

/** Before the target unit of a conversion: `5\,\mathrm{cm} \blacktriangleright \mathrm{in}`. */
export const CONVERT = ' \\blacktriangleright ';

/**
 * A function's template with MathLive's placeholders in its empty slots: `\sin()` becomes
 * `\sin\left(#?\right)`, `\operatorname{ggT}(;)` two of them, `\int_{}^{}` the bounds.
 */
export const withPlaceholders = (latex: string) =>
  latex
    .replace('\\lfloor\\rfloor', '\\left\\lfloor #?\\right\\rfloor')
    .replace('\\lceil\\rceil', '\\left\\lceil #?\\right\\rceil')
    .replace('\\lim_{x\\to}', '\\lim_{x\\to #?}')
    .replace(/^\|\|$/, '\\left|#?\\right|')
    .replaceAll('(;)', '\\left(#?;#?\\right)')
    .replaceAll('()', '\\left(#?\\right)')
    .replaceAll('[]', '[#?]')
    .replaceAll('{}', '{#?}');

/** What a hit puts in: a unit as the target of a conversion, or alone with Alt. */
export function hitLatex(hit: LibraryHit, unitAlone: boolean): string {
  switch (hit.kind) {
    case 'unit':
      return unitAlone ? hit.latex : CONVERT + hit.latex;
    case 'function':
      return withPlaceholders(hit.latex);
    case 'prefix':
      return `\\mathrm{${hit.latex.trim()}}`;
    default:
      return hit.latex;
  }
}

const KIND_LABEL = { constant: 'quick.constant', unit: 'quick.unit', function: 'quick.function', prefix: 'quick.prefix' } as const;

/**
 * The engine's ranked hits for `query`, again whenever it or the language changes, with the
 * query they belong to: right after a keystroke they are still the last query's.
 */
export function useLibrarySearch(query: string, limit?: number): { hits: LibraryHit[]; forQuery: string } {
  const { language } = useT();
  const [found, setFound] = useState<{ hits: LibraryHit[]; forQuery: string }>({ hits: [], forQuery: '' });
  useEffect(() => {
    let current = true;
    if (!query.trim()) setFound({ hits: [], forQuery: query });
    else api.searchLibrary(query, limit).then((hits) => current && setFound({ hits, forQuery: query }), () => {});
    return () => {
      current = false;
    };
  }, [query, limit, language]);
  return found;
}

/** One hit: symbol, name, value or category, and what kind it is. */
export function HitRow(props: {
  hit: LibraryHit;
  selected?: boolean;
  onPick: (event: MouseEvent) => void;
  onHover?: () => void;
}) {
  const { t } = useT();
  const { hit } = props;
  return (
    <button
      className={props.selected ? 'hit selected' : 'hit'}
      title={hit.kind === 'unit' ? t('library.unit.title') : hit.name}
      onClick={props.onPick}
      onMouseMove={props.onHover}
    >
      <Katex className="hit-symbol" latex={hit.latex} />
      <span className="hit-name">{hit.name}</span>
      <span className="hit-kind">{t(KIND_LABEL[hit.kind])}</span>
      {hit.kind === 'unit' || hit.kind === 'function' ? (
        <span className="hit-detail">{hit.detail}</span>
      ) : (
        <Katex className="hit-detail" latex={hit.detail} />
      )}
    </button>
  );
}
