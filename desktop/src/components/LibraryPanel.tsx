import { useEffect, useMemo, useState } from 'react';
import { api, type ConstantCategory, type UnitCategory } from '../api';
import { useT } from '../i18n';
import { Katex } from './Katex';

/** Before the target unit of a conversion: `5\,\mathrm{cm} \blacktriangleright \mathrm{in}`. */
const CONVERT = ' \\blacktriangleright ';

/**
 * The constants and units a calculator has on CONST and CONV: a click puts a constant, or a
 * conversion to a unit, in at the cursor. Both lists come from the engine in the set language.
 */
export function LibraryPanel(props: { onInsert: (latex: string) => void; unitsKnown: boolean }) {
  const { t, language } = useT();
  const [tab, setTab] = useState<'constants' | 'units'>('constants');
  const [constants, setConstants] = useState<ConstantCategory[]>([]);
  const [units, setUnits] = useState<UnitCategory[]>([]);
  const [filter, setFilter] = useState('');

  useEffect(() => {
    api.constants().then(setConstants, () => {});
    api.conversionUnits().then(setUnits, () => {});
  }, [language]);

  const needle = filter.trim().toLowerCase();
  const shownConstants = useMemo(
    () =>
      constants
        .map((category) => ({
          ...category,
          constants: category.constants.filter(
            (constant) => !needle || constant.name.toLowerCase().includes(needle) || constant.key.toLowerCase().includes(needle),
          ),
        }))
        .filter((category) => category.constants.length > 0),
    [constants, needle],
  );
  const shownUnits = useMemo(
    () =>
      units
        .map((category) =>
          !needle || category.name.toLowerCase().includes(needle)
            ? category
            : { ...category, units: category.units.filter((unit) => unit.toLowerCase().includes(needle)) },
        )
        .filter((category) => category.units.length > 0),
    [units, needle],
  );

  return (
    <div className="library">
      <div className="library-head">
        <div className="segmented">
          <button className={tab === 'constants' ? 'segment active' : 'segment'} onClick={() => setTab('constants')}>
            {t('library.constants')}
          </button>
          <button className={tab === 'units' ? 'segment active' : 'segment'} onClick={() => setTab('units')}>
            {t('library.units')}
          </button>
        </div>
        <input
          className="library-filter"
          placeholder={t('library.filter')}
          value={filter}
          onChange={(event) => setFilter(event.target.value)}
        />
        {!props.unitsKnown && <p className="setting-hint">{t('library.physicsOnly')}</p>}
        {tab === 'units' && <p className="setting-hint">{t('library.units.hint')}</p>}
      </div>

      {tab === 'constants'
        ? shownConstants.map((category) => (
            <section className="library-group" key={category.category}>
              <h3>{category.name}</h3>
              {category.constants.map((constant) => (
                <button
                  className="library-item"
                  key={constant.key}
                  title={constant.name}
                  onClick={() => props.onInsert(constant.latex)}
                >
                  <Katex className="library-symbol" latex={constant.latex} />
                  <span className="library-name">{constant.name}</span>
                  <Katex className="library-value" latex={constant.value} />
                </button>
              ))}
            </section>
          ))
        : shownUnits.map((category) => (
            <section className="library-group" key={category.name}>
              <h3>{category.name}</h3>
              <div className="library-units">
                {category.units.map((unit) => (
                  <button
                    className="library-unit"
                    key={unit}
                    title={t('library.unit.title')}
                    onClick={(event) => props.onInsert(event.altKey ? unit : CONVERT + unit)}
                  >
                    <Katex latex={unit} />
                  </button>
                ))}
              </div>
            </section>
          ))}
    </div>
  );
}
