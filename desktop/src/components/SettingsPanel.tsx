import { useEffect, useState } from 'react';
import type { BinomialForm, ConditionalIdentities, DenominatorForm, Language, Mode, QuadraticFormula, Settings, StepDetail } from '../api';
import { LANGUAGE_NAMES, useT, type Translate } from '../i18n';

interface Choice<T extends string> {
  value: T;
  label: string;
}

export const modeChoices = (t: Translate): Choice<Mode>[] => [
  { value: 'algebra', label: t('mode.algebra') },
  { value: 'physics', label: t('mode.physics') },
  { value: 'imaginary', label: t('mode.imaginary') },
  { value: 'analysis', label: t('mode.analysis') },
];

const LANGUAGE_CHOICES: Choice<Language>[] = (['en', 'de'] as const).map((value) => ({
  value,
  label: LANGUAGE_NAMES[value],
}));

const binomialChoices = (t: Translate): Choice<BinomialForm>[] => [
  { value: 'auto', label: t('choice.auto') },
  { value: 'expanded', label: t('choice.expanded') },
  { value: 'binomial', label: t('choice.binomial') },
];

const quadraticChoices = (t: Translate): Choice<QuadraticFormula>[] => [
  { value: 'auto', label: t('choice.auto') },
  { value: 'pq', label: t('choice.pq') },
  { value: 'abc', label: t('choice.abc') },
];

const denominatorChoices = (t: Translate): Choice<DenominatorForm>[] => [
  { value: 'auto', label: t('choice.auto') },
  { value: 'expanded', label: t('choice.expanded') },
  { value: 'factored', label: t('choice.factored') },
];

const conditionChoices = (t: Translate): Choice<ConditionalIdentities>[] => [
  { value: 'withCondition', label: t('choice.withCondition') },
  { value: 'alwaysValid', label: t('choice.alwaysValid') },
];

const detailChoices = (t: Translate): Choice<StepDetail>[] => [
  { value: 'none', label: t('detail.none') },
  { value: 'brief', label: t('detail.brief') },
  { value: 'standard', label: t('detail.standard') },
  { value: 'detailed', label: t('detail.detailed') },
  { value: 'maximal', label: t('detail.maximal') },
];

/** So sieht die Engine ohne Einstellungsdatei aus. */
export const DEFAULT_SETTINGS: Settings = {
  mode: 'physics',
  precision: 50,
  binomialForm: 'auto',
  completeSquare: true,
  denominatorForm: 'auto',
  conditionalIdentities: 'withCondition',
  exactRoots: false,
  aAndYAsVariables: true,
  letterProducts: true,
  extraUnitLetters: false,
  language: 'en',
  quadraticFormula: 'auto',
  stepDetail: 'standard',
  cancelMarks: true,
};

export function Segmented<T extends string>(props: { choices: Choice<T>[]; value: T; onChange: (value: T) => void }) {
  return (
    <div className="segmented">
      {props.choices.map((choice) => (
        <button
          key={choice.value}
          className={choice.value === props.value ? 'segment active' : 'segment'}
          onClick={() => props.onChange(choice.value)}
        >
          {choice.label}
        </button>
      ))}
    </div>
  );
}

/** A dropdown for choices too many or too long to fit side by side in the panel. */
function Dropdown<T extends string>(props: { choices: Choice<T>[]; value: T; onChange: (value: T) => void }) {
  return (
    <select className="setting-select" value={props.value} onChange={(event) => props.onChange(event.target.value as T)}>
      {props.choices.map((choice) => (
        <option key={choice.value} value={choice.value}>
          {choice.label}
        </option>
      ))}
    </select>
  );
}

function Toggle(props: { checked: boolean; label: string; hint: string; onChange: (checked: boolean) => void }) {
  return (
    <label className="setting-toggle">
      <input type="checkbox" checked={props.checked} onChange={(event) => props.onChange(event.target.checked)} />
      <span>
        {props.label}
        <span className="setting-hint">{props.hint}</span>
      </span>
    </label>
  );
}

/**
 * Wie Eingaben gelesen und Ergebnisse geschrieben werden. Jede Änderung geht sofort an die
 * Engine, die sie prüft und speichert; ein Fehler bleibt hier im Panel stehen.
 */
export function SettingsPanel(props: { settings: Settings; error: string | null; onChange: (settings: Settings) => void }) {
  const { settings, onChange } = props;
  const { t } = useT();
  const set = <K extends keyof Settings>(key: K, value: Settings[K]) => onChange({ ...settings, [key]: value });

  // Die Genauigkeit wird erst beim Verlassen des Feldes übernommen, nicht bei jeder Ziffer.
  const [precision, setPrecision] = useState(String(settings.precision));
  useEffect(() => setPrecision(String(settings.precision)), [settings.precision]);
  const commitPrecision = () => {
    const value = Number(precision);
    if (precision.trim() !== '' && Number.isInteger(value) && value !== settings.precision) set('precision', value);
    else setPrecision(String(settings.precision));
  };

  return (
    <div className="settings">
      <section className="setting">
        <h3>{t('settings.language')}</h3>
        <Segmented choices={LANGUAGE_CHOICES} value={settings.language} onChange={(v) => set('language', v)} />
      </section>

      <section className="setting">
        <h3>{t('settings.mode')}</h3>
        <Dropdown choices={modeChoices(t)} value={settings.mode} onChange={(v) => set('mode', v)} />
        <p className="setting-hint">{t(`mode.${settings.mode}.hint`)}</p>
      </section>

      <section className="setting">
        <h3>{t('settings.detail')}</h3>
        <Dropdown choices={detailChoices(t)} value={settings.stepDetail} onChange={(v) => set('stepDetail', v)} />
        <p className="setting-hint">{t(`detail.${settings.stepDetail}.hint`)}</p>
        <Toggle
          checked={settings.cancelMarks}
          label={t('settings.cancel')}
          hint={t('settings.cancel.hint')}
          onChange={(v) => set('cancelMarks', v)}
        />
      </section>

      {settings.mode === 'analysis' && (
        <section className="setting">
          <h3>{t('settings.quadratic')}</h3>
          <Segmented
            choices={quadraticChoices(t)}
            value={settings.quadraticFormula}
            onChange={(v) => set('quadraticFormula', v)}
          />
          <p className="setting-hint">{t('settings.quadratic.hint')}</p>
        </section>
      )}

      <section className="setting">
        <h3>{t('settings.precision')}</h3>
        <div className="setting-precision">
          <input
            type="number"
            min={1}
            max={10000}
            value={precision}
            onChange={(event) => setPrecision(event.target.value)}
            onBlur={commitPrecision}
            onKeyDown={(event) => event.key === 'Enter' && commitPrecision()}
          />
          <span className="setting-hint">{t('settings.digits')}</span>
        </div>
      </section>

      <section className="setting">
        <h3>{t('settings.binomial')}</h3>
        <Segmented choices={binomialChoices(t)} value={settings.binomialForm} onChange={(v) => set('binomialForm', v)} />
        <p className="setting-hint">
          {t('settings.binomial.hint')}
        </p>
        <Toggle
          checked={settings.completeSquare}
          label={t('settings.square')}
          hint={t('settings.square.hint')}
          onChange={(v) => set('completeSquare', v)}
        />
      </section>

      <section className="setting">
        <h3>{t('settings.denominator')}</h3>
        <Segmented
          choices={denominatorChoices(t)}
          value={settings.denominatorForm}
          onChange={(v) => set('denominatorForm', v)}
        />
        <p className="setting-hint">{t('settings.denominator.hint')}</p>
      </section>

      <section className="setting">
        <h3>{t('settings.conditions')}</h3>
        <Segmented
          choices={conditionChoices(t)}
          value={settings.conditionalIdentities}
          onChange={(v) => set('conditionalIdentities', v)}
        />
        <p className="setting-hint">{t('settings.conditions.hint')}</p>
      </section>

      <section className="setting">
        <h3>{t('settings.exact')}</h3>
        <Toggle
          checked={settings.exactRoots}
          label={t('settings.roots')}
          hint={t('settings.roots.hint')}
          onChange={(v) => set('exactRoots', v)}
        />
      </section>

      <section className="setting">
        <h3>{t('settings.input')}</h3>
        <Toggle
          checked={settings.aAndYAsVariables}
          label={t('settings.ay')}
          hint={t('settings.ay.hint')}
          onChange={(v) => set('aAndYAsVariables', v)}
        />
        <Toggle
          checked={settings.letterProducts}
          label={t('settings.products')}
          hint={t('settings.products.hint')}
          onChange={(v) => set('letterProducts', v)}
        />
        <Toggle
          checked={settings.extraUnitLetters}
          label={t('settings.letters')}
          hint={t('settings.letters.hint')}
          onChange={(v) => set('extraUnitLetters', v)}
        />
      </section>

      {props.error && <p className="setting-error">{props.error}</p>}

      <button className="setting-reset" onClick={() => onChange({ ...DEFAULT_SETTINGS, language: settings.language })}>
        {t('settings.reset')}
      </button>
    </div>
  );
}
