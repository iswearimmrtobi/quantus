import { useEffect, useState, type KeyboardEvent, type MouseEvent } from 'react';
import { api, type ConstantCategory, type UnitCategory } from '../api';
import { useT } from '../i18n';
import { Katex } from './Katex';
import { CONVERT, hitLatex, HitRow, useLibrarySearch } from './LibraryHits';

type Tab = 'constants' | 'units';

/**
 * The constants and units a calculator has on CONST and CONV, in a window like the settings.
 * A click puts a constant, or a conversion to a unit, in at the cursor and closes the window;
 * with Shift it stays open for more. Typing in the filter lists the engine's ranked hits from
 * both, and Enter takes the first.
 */
export function LibraryDialog(props: {
  unitsKnown: boolean;
  /** `keepOpen` with Shift held. */
  onInsert: (latex: string, keepOpen: boolean) => void;
  onClose: () => void;
}) {
  const { t, language } = useT();
  const [tab, setTab] = useState<Tab>('constants');
  const [constants, setConstants] = useState<ConstantCategory[]>([]);
  const [units, setUnits] = useState<UnitCategory[]>([]);
  const [filter, setFilter] = useState('');
  const hits = useLibrarySearch(filter, 40);

  useEffect(() => {
    api.constants().then(setConstants, () => {});
    api.conversionUnits().then(setUnits, () => {});
  }, [language]);

  const pick = (latex: string) => (event: MouseEvent) => props.onInsert(latex, event.shiftKey);

  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key === 'Escape') {
      event.stopPropagation();
      props.onClose();
    }
  };

  const onFilterKey = (event: KeyboardEvent) => {
    if (event.key === 'Enter' && hits.length > 0) {
      event.preventDefault();
      props.onInsert(hitLatex(hits[0], event.altKey), event.shiftKey);
    }
  };

  const searching = filter.trim() !== '';

  return (
    <div className="dialog-backdrop" onMouseDown={props.onClose}>
      <div
        className="dialog settings-dialog library-dialog"
        role="dialog"
        aria-label={t('library')}
        onMouseDown={(event) => event.stopPropagation()}
        onKeyDown={onKeyDown}
      >
        <div className="settings-head">
          <h3>{t('library')}</h3>
          <button className="settings-close" title={t('settings.close')} onClick={props.onClose}>
            ✕
          </button>
        </div>
        <nav className="settings-nav">
          {(['constants', 'units'] as const).map((name) => (
            <button
              key={name}
              className={!searching && name === tab ? 'settings-cat active' : 'settings-cat'}
              onClick={() => {
                setTab(name);
                setFilter('');
              }}
            >
              {t(`library.${name}`)}
            </button>
          ))}
        </nav>
        <div className="settings-body library">
          <div className="library-head">
            <input
              className="library-filter"
              autoFocus
              placeholder={t('library.filter')}
              value={filter}
              onChange={(event) => setFilter(event.target.value)}
              onKeyDown={onFilterKey}
            />
            {!props.unitsKnown && <p className="setting-hint">{t('library.physicsOnly')}</p>}
            <p className="setting-hint">{t('library.shift')}</p>
            {(searching || tab === 'units') && <p className="setting-hint">{t('library.units.hint')}</p>}
          </div>

          {searching ? (
            hits.length === 0 ? (
              <p className="panel-empty">{t('quick.empty')}</p>
            ) : (
              <div className="hits">
                {hits.map((hit) => (
                  <HitRow
                    key={`${hit.kind}:${hit.latex}`}
                    hit={hit}
                    onPick={(event) => props.onInsert(hitLatex(hit, event.altKey), event.shiftKey)}
                  />
                ))}
              </div>
            )
          ) : tab === 'constants' ? (
            constants.map((category) => (
              <section className="library-group" key={category.category}>
                <h3>{category.name}</h3>
                <div className="library-constants">
                  {category.constants.map((constant) => (
                    <button
                      className="library-item"
                      key={constant.key}
                      title={constant.name}
                      onClick={pick(constant.latex)}
                    >
                      <Katex className="library-symbol" latex={constant.latex} />
                      <span className="library-name">{constant.name}</span>
                      <Katex className="library-value" latex={constant.value} />
                    </button>
                  ))}
                </div>
              </section>
            ))
          ) : (
            units.map((category) => (
              <section className="library-group" key={category.name}>
                <h3>{category.name}</h3>
                <div className="library-units">
                  {category.units.map((unit) => (
                    <button
                      className="library-unit"
                      key={unit.latex}
                      title={t('library.unit.title')}
                      onClick={(event) => props.onInsert(event.altKey ? unit.latex : CONVERT + unit.latex, event.shiftKey)}
                    >
                      <Katex latex={unit.latex} />
                      <span className="library-unit-name">{unit.name}</span>
                    </button>
                  ))}
                </div>
              </section>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
