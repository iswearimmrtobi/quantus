import { useEffect, useState, type KeyboardEvent } from 'react';
import { api } from '../api';
import { useT } from '../i18n';
import { hitLatex, HitRow, useLibrarySearch } from './LibraryHits';

/**
 * One line to find a constant or unit by name, like a launcher: type, Enter, done. The first
 * hit is chosen from the start; ↑↓ choose another, Alt+Enter puts a unit in alone.
 */
export function QuickSearch(props: { onInsert: (latex: string) => void; onClose: () => void }) {
  const { t } = useT();
  const [query, setQuery] = useState('');
  const [selected, setSelected] = useState(0);
  const { hits, forQuery } = useLibrarySearch(query, 8);

  useEffect(() => setSelected(0), [hits]);

  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key === 'Escape') {
      event.stopPropagation();
      props.onClose();
    } else if (event.key === 'ArrowDown' || (event.key === 'Tab' && !event.shiftKey)) {
      event.preventDefault();
      if (hits.length) setSelected((index) => (index + 1) % hits.length);
    } else if (event.key === 'ArrowUp' || (event.key === 'Tab' && event.shiftKey)) {
      event.preventDefault();
      if (hits.length) setSelected((index) => (index - 1 + hits.length) % hits.length);
    } else if (event.key === 'Enter') {
      event.preventDefault();
      const unitAlone = event.altKey;
      if (forQuery === query) {
        const hit = hits[selected];
        if (hit) props.onInsert(hitLatex(hit, unitAlone));
      } else if (query.trim()) {
        // Typed and pressed Enter faster than the hits came: take the first of the fresh ones.
        api.searchLibrary(query, 1).then(([hit]) => hit && props.onInsert(hitLatex(hit, unitAlone)), () => {});
      }
    }
  };

  return (
    <div className="dialog-backdrop quick-backdrop" onMouseDown={props.onClose}>
      <div
        className="dialog quick-search"
        role="dialog"
        aria-label={t('quick')}
        onMouseDown={(event) => event.stopPropagation()}
        onKeyDown={onKeyDown}
      >
        <input
          className="quick-input"
          autoFocus
          spellCheck={false}
          placeholder={t('quick.placeholder')}
          value={query}
          onChange={(event) => setQuery(event.target.value)}
        />
        {query.trim() !== '' &&
          (hits.length === 0 ? (
            <p className="quick-empty">{t('quick.empty')}</p>
          ) : (
            <div className="hits">
              {hits.map((hit, index) => (
                <HitRow
                  key={`${hit.kind}:${hit.latex}`}
                  hit={hit}
                  selected={index === selected}
                  onHover={() => setSelected(index)}
                  onPick={(event) => props.onInsert(hitLatex(hit, event.altKey))}
                />
              ))}
            </div>
          ))}
        <p className="quick-hint">{t('quick.hint')}</p>
      </div>
    </div>
  );
}
