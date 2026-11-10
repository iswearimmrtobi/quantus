import { useCallback, useEffect, useState, type KeyboardEvent, type ReactNode } from 'react';
import type { AngleUnit, BinomialForm, ComplexForm, ConditionalIdentities, DenominatorForm, Elimination, FractionForm, Language, Mode, NumberFormat, QuadraticFormula, Settings, StarDelta, StepDetail, Themes, UnknownNames } from '../api';
import { api } from '../api';
import { SYSTEM_THEME } from '../theme';
import { LANGUAGES, LANGUAGE_NAMES, useT, type Translate } from '../i18n';

interface Choice<T extends string> {
  value: T;
  label: string;
  /** Shown on hover. */
  hint?: string;
}

/** With the imaginary unit switched on, the linear systems mode reads "… + 𝑖". */
export const modeChoices = (t: Translate, linearComplex: boolean): Choice<Mode>[] =>
  (['algebra', 'physics', 'imaginary', 'electronics', 'analysis', 'linear', 'kirchhoff', 'geometry', 'baseN', 'verify'] as const).map(
    (value) => ({
      value,
      label: value === 'linear' && linearComplex ? `${t('mode.linear')} + 𝑖` : t(`mode.${value}`),
      hint: t(`mode.${value}.hint`),
    }),
  );

const LANGUAGE_CHOICES: Choice<Language>[] = LANGUAGES.map((value) => ({
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

const unknownNameChoices = (t: Translate): Choice<UnknownNames>[] => [
  { value: 'indexed', label: t('choice.indexed') },
  { value: 'letters', label: t('choice.letters') },
];

const eliminationChoices = (t: Translate): Choice<Elimination>[] => [
  { value: 'gauss', label: t('choice.gauss') },
  { value: 'gaussJordan', label: t('choice.gaussJordan') },
];

const starDeltaChoices = (t: Translate): Choice<StarDelta>[] => [
  { value: 'nodeRule', label: t('choice.nodeRule') },
  { value: 'deltaToStar', label: t('choice.deltaToStar') },
  { value: 'starToDelta', label: t('choice.starToDelta') },
];

const angleChoices = (t: Translate): Choice<AngleUnit>[] => [
  { value: 'degree', label: t('choice.degree') },
  { value: 'radian', label: t('choice.radian') },
];

type FormatStyle = NumberFormat['style'];

const formatChoices = (t: Translate): Choice<FormatStyle>[] => [
  { value: 'auto', label: t('choice.auto') },
  { value: 'norm1', label: 'Norm 1' },
  { value: 'norm2', label: 'Norm 2' },
  { value: 'fix', label: 'Fix' },
  { value: 'sci', label: 'Sci' },
];

const fractionChoices = (t: Translate): Choice<FractionForm>[] => [
  { value: 'improper', label: t('choice.improper') },
  { value: 'mixed', label: t('choice.mixed') },
];

/** Notation, the same in every language. */
const COMPLEX_CHOICES: Choice<ComplexForm>[] = [
  { value: 'rectangular', label: 'a + bi' },
  { value: 'polar', label: 'r∠θ' },
];

/** Fix and Sci keep the digits they had, or start at 2. */
const withStyle = (format: NumberFormat, style: FormatStyle): NumberFormat =>
  style === 'fix' || style === 'sci' ? { style, digits: 'digits' in format ? format.digits : 2 } : { style };

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
  elimination: 'gauss',
  starDelta: 'nodeRule',
  unknownNames: 'indexed',
  linearComplex: false,
  angleUnit: 'radian',
  numberFormat: { style: 'auto' },
  decimalPrefixes: false,
  digitGrouping: false,
  fractionForm: 'improper',
  recurring: true,
  complexForm: 'rectangular',
  numberBase: 'dec',
  theme: SYSTEM_THEME,
};

export function Segmented<T extends string>(props: { choices: Choice<T>[]; value: T; onChange: (value: T) => void }) {
  return (
    <div className="segmented">
      {props.choices.map((choice) => (
        <button
          key={choice.value}
          className={choice.value === props.value ? 'segment active' : 'segment'}
          title={choice.hint}
          onClick={() => props.onChange(choice.value)}
        >
          {choice.label}
        </button>
      ))}
    </div>
  );
}

/** A dropdown for choices too many or too long to fit side by side. */
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

type Category = 'general' | 'numbers' | 'algebra' | 'steps' | 'input';

const CATEGORIES: Category[] = ['general', 'numbers', 'algebra', 'steps', 'input'];

/** The category the dialog was left on; only a convenience, so a blocked storage is fine. */
const CATEGORY_KEY = 'quantus.settingsCategory';

function storedCategory(): Category {
  try {
    const stored = localStorage.getItem(CATEGORY_KEY);
    return CATEGORIES.includes(stored as Category) ? (stored as Category) : 'general';
  } catch {
    return 'general';
  }
}

/**
 * Wie Eingaben gelesen und Ergebnisse geschrieben werden, nach Bereichen getrennt. Jede Änderung
 * geht sofort an die Engine, die sie prüft und speichert; ein Fehler bleibt hier im Dialog stehen.
 */
export function SettingsDialog(props: {
  settings: Settings;
  error: string | null;
  /** Why the chosen theme is not shown. */
  themeError: string | null;
  onChange: (settings: Settings) => void;
  /** Reads the theme files again after the user changed one. */
  onReloadTheme: () => void;
  onClose: () => void;
}) {
  const { settings, onChange } = props;
  const { t } = useT();
  const set = <K extends keyof Settings>(key: K, value: Settings[K]) => onChange({ ...settings, [key]: value });

  const [category, setCategory] = useState(storedCategory);
  const choose = (next: Category) => {
    setCategory(next);
    try {
      localStorage.setItem(CATEGORY_KEY, next);
    } catch {
      // Then it opens on General next time.
    }
  };

  // Die Genauigkeit wird erst beim Verlassen des Feldes übernommen, nicht bei jeder Ziffer.
  const [precision, setPrecision] = useState(String(settings.precision));
  useEffect(() => setPrecision(String(settings.precision)), [settings.precision]);
  const commitPrecision = () => {
    const value = Number(precision);
    if (precision.trim() !== '' && Number.isInteger(value) && value !== settings.precision) set('precision', value);
    else setPrecision(String(settings.precision));
  };

  // The digits of Fix and Sci, taken over the same way.
  const format = settings.numberFormat;
  const [digits, setDigits] = useState('digits' in format ? String(format.digits) : '');
  useEffect(() => setDigits('digits' in format ? String(format.digits) : ''), [format]);
  const commitDigits = () => {
    const value = Number(digits);
    if (!('digits' in format)) return;
    if (digits.trim() !== '' && Number.isInteger(value) && value !== format.digits) set('numberFormat', { ...format, digits: value });
    else setDigits(String(format.digits));
  };

  // The themes folder is read again whenever the dialog opens and on Reload.
  const [themes, setThemes] = useState<Themes | null>(null);
  const [themeFailure, setThemeFailure] = useState<string | null>(null);
  const loadThemes = useCallback(() => {
    api.themes().then(setThemes, (failure) => setThemeFailure(String(failure)));
  }, []);
  useEffect(loadThemes, [loadThemes, settings.language]);
  const reloadThemes = () => {
    setThemeFailure(null);
    loadThemes();
    props.onReloadTheme();
  };
  const openThemesFolder = () => {
    api.openThemesFolder().catch((failure) => setThemeFailure(String(failure)));
  };
  const chosenTheme = themes?.themes.find((theme) => theme.id === settings.theme);

  /** Settings that only one mode reads say which. */
  const onlyIn = (mode: Mode) => <span className="setting-only">{t('settings.onlyIn', { mode: t(`mode.${mode}`) })}</span>;

  const general = (
    <>
      <section className="setting">
        <h3>{t('settings.language')}</h3>
        <Segmented choices={LANGUAGE_CHOICES} value={settings.language} onChange={(v) => set('language', v)} />
      </section>

      <section className="setting">
        <h3>{t('settings.theme')}</h3>
        <select className="setting-select" value={settings.theme} onChange={(event) => set('theme', event.target.value)}>
          <option value={SYSTEM_THEME}>{t('settings.theme.system')}</option>
          {themes?.themes.map((theme) => (
            <option key={theme.id} value={theme.id} disabled={theme.error !== null} title={theme.error ?? undefined}>
              {theme.error === null ? theme.name : t('settings.theme.broken', { name: theme.name })}
            </option>
          ))}
          {themes && settings.theme !== SYSTEM_THEME && !chosenTheme && <option value={settings.theme}>{settings.theme}</option>}
        </select>
        <div className="setting-actions">
          <button className="setting-reset" onClick={openThemesFolder}>
            {t('settings.theme.folder')}
          </button>
          <button className="setting-reset" onClick={reloadThemes}>
            {t('settings.theme.reload')}
          </button>
        </div>
        <p className="setting-hint">{t('settings.theme.hint', { dir: themes?.dir ?? '…' })}</p>
        {themes?.themes
          .filter((theme) => theme.error !== null)
          .map((theme) => (
            <p key={theme.id} className="setting-error">
              {theme.id}.toml: {theme.error}
            </p>
          ))}
        {chosenTheme && chosenTheme.warnings.length > 0 && (
          <p className="setting-hint">{t('settings.theme.warnings', { warnings: chosenTheme.warnings.join('; ') })}</p>
        )}
        {(themeFailure ?? props.themeError) && <p className="setting-error">{themeFailure ?? props.themeError}</p>}
      </section>

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
        <h3>{t('settings.angle')}</h3>
        <Segmented choices={angleChoices(t)} value={settings.angleUnit} onChange={(v) => set('angleUnit', v)} />
        <p className="setting-hint">{t('settings.angle.hint')}</p>
      </section>
    </>
  );

  const numbers = (
    <>
      <section className="setting">
        <h3>{t('settings.format')}</h3>
        <Segmented
          choices={formatChoices(t)}
          value={format.style}
          onChange={(v) => set('numberFormat', withStyle(format, v))}
        />
        {'digits' in format && (
          <div className="setting-precision">
            <input
              type="number"
              min={0}
              max={9}
              value={digits}
              onChange={(event) => setDigits(event.target.value)}
              onBlur={commitDigits}
              onKeyDown={(event) => event.key === 'Enter' && commitDigits()}
            />
            <span className="setting-hint">{t(format.style === 'fix' ? 'settings.format.places' : 'settings.format.significant')}</span>
          </div>
        )}
        <p className="setting-hint">{t(`settings.format.${format.style}.hint`)}</p>
        <Toggle
          checked={settings.decimalPrefixes}
          label={t('settings.prefixes')}
          hint={t('settings.prefixes.hint')}
          onChange={(v) => set('decimalPrefixes', v)}
        />
        <Toggle
          checked={settings.digitGrouping}
          label={t('settings.grouping')}
          hint={t('settings.grouping.hint')}
          onChange={(v) => set('digitGrouping', v)}
        />
      </section>

      <section className="setting">
        <h3>{t('settings.fractions')}</h3>
        <Segmented choices={fractionChoices(t)} value={settings.fractionForm} onChange={(v) => set('fractionForm', v)} />
        <p className="setting-hint">{t('settings.fractions.hint')}</p>
        <Toggle
          checked={settings.recurring}
          label={t('settings.recurring')}
          hint={t('settings.recurring.hint')}
          onChange={(v) => set('recurring', v)}
        />
      </section>

      <section className="setting">
        <h3>{t('settings.complex')}</h3>
        <Segmented choices={COMPLEX_CHOICES} value={settings.complexForm} onChange={(v) => set('complexForm', v)} />
        <p className="setting-hint">
          {t('settings.complex.hint')} {onlyIn('imaginary')}
        </p>
      </section>
    </>
  );

  const algebra = (
    <>
      <section className="setting">
        <h3>{t('settings.binomial')}</h3>
        <Segmented choices={binomialChoices(t)} value={settings.binomialForm} onChange={(v) => set('binomialForm', v)} />
        <p className="setting-hint">{t('settings.binomial.hint')}</p>
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
    </>
  );

  const steps = (
    <>
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

      <section className="setting">
        <h3>{t('settings.elimination')}</h3>
        <Segmented choices={eliminationChoices(t)} value={settings.elimination} onChange={(v) => set('elimination', v)} />
        <p className="setting-hint">{t(`settings.elimination.${settings.elimination}.hint`)}</p>
      </section>

      <section className="setting">
        <h3>{t('settings.starDelta')}</h3>
        <Segmented choices={starDeltaChoices(t)} value={settings.starDelta} onChange={(v) => set('starDelta', v)} />
        <p className="setting-hint">
          {t(`settings.starDelta.${settings.starDelta}.hint`)} {onlyIn('kirchhoff')}
        </p>
      </section>

      <section className="setting">
        <h3>{t('settings.quadratic')}</h3>
        <Segmented
          choices={quadraticChoices(t)}
          value={settings.quadraticFormula}
          onChange={(v) => set('quadraticFormula', v)}
        />
        <p className="setting-hint">
          {t('settings.quadratic.hint')} {onlyIn('analysis')}
        </p>
      </section>

      <section className="setting">
        <h3>{t('settings.unknownNames')}</h3>
        <Segmented
          choices={unknownNameChoices(t)}
          value={settings.unknownNames}
          onChange={(v) => set('unknownNames', v)}
        />
        <p className="setting-hint">
          {t('settings.unknownNames.hint')} {onlyIn('linear')}
        </p>
      </section>
    </>
  );

  const input = (
    <section className="setting">
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
  );

  const pages: Record<Category, ReactNode> = { general, numbers, algebra, steps, input };

  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key === 'Escape') {
      event.stopPropagation();
      props.onClose();
    }
  };

  return (
    <div className="dialog-backdrop" onMouseDown={props.onClose}>
      <div
        className="dialog settings-dialog"
        role="dialog"
        aria-label={t('settings')}
        onMouseDown={(event) => event.stopPropagation()}
        onKeyDown={onKeyDown}
      >
        <div className="settings-head">
          <h3>{t('settings')}</h3>
          <button className="settings-close" title={t('settings.close')} onClick={props.onClose}>
            ✕
          </button>
        </div>
        <nav className="settings-nav">
          {CATEGORIES.map((name) => (
            <button
              key={name}
              autoFocus={name === category}
              className={name === category ? 'settings-cat active' : 'settings-cat'}
              onClick={() => choose(name)}
            >
              {t(`settings.cat.${name}`)}
            </button>
          ))}
        </nav>
        <div className="settings-body">{pages[category]}</div>
        <div className="settings-foot">
          {props.error && <p className="setting-error">{props.error}</p>}
          <button className="setting-reset" onClick={() => onChange({ ...DEFAULT_SETTINGS, language: settings.language, theme: settings.theme })}>
            {t('settings.reset')}
          </button>
        </div>
      </div>
    </div>
  );
}
