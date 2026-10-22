import { useEffect, useState } from 'react';
import type { BinomialForm, DenominatorForm, Settings } from '../api';

interface Choice<T extends string> {
  value: T;
  label: string;
}

const BINOMIAL_CHOICES: Choice<BinomialForm>[] = [
  { value: 'auto', label: 'Automatisch' },
  { value: 'expanded', label: 'Ausmultipliziert' },
  { value: 'binomial', label: 'Als Binom' },
];

const DENOMINATOR_CHOICES: Choice<DenominatorForm>[] = [
  { value: 'auto', label: 'Automatisch' },
  { value: 'expanded', label: 'Ausmultipliziert' },
  { value: 'factored', label: 'Faktorisiert' },
];

/** So sieht die Engine ohne Einstellungsdatei aus. */
export const DEFAULT_SETTINGS: Settings = {
  precision: 50,
  binomialForm: 'auto',
  completeSquare: true,
  denominatorForm: 'auto',
  aAndYAsVariables: true,
  letterProducts: true,
};

function Segmented<T extends string>(props: { choices: Choice<T>[]; value: T; onChange: (value: T) => void }) {
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
        <h3>Genauigkeit</h3>
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
          <span className="setting-hint">signifikante Stellen</span>
        </div>
      </section>

      <section className="setting">
        <h3>Binomische Formeln</h3>
        <Segmented choices={BINOMIAL_CHOICES} value={settings.binomialForm} onChange={(v) => set('binomialForm', v)} />
        <p className="setting-hint">
          Automatisch: als Binom, außer die Eingabe war schon eins — dann ausmultipliziert.
        </p>
        <Toggle
          checked={settings.completeSquare}
          label="Quadratische Ergänzung"
          hint="x² + 8x + 11 wird zu (x + 4)² − 5"
          onChange={(v) => set('completeSquare', v)}
        />
      </section>

      <section className="setting">
        <h3>Nenner</h3>
        <Segmented
          choices={DENOMINATOR_CHOICES}
          value={settings.denominatorForm}
          onChange={(v) => set('denominatorForm', v)}
        />
        <p className="setting-hint">Automatisch: faktorisiert, wenn er in mehrere Faktoren zerfällt.</p>
      </section>

      <section className="setting">
        <h3>Eingabe</h3>
        <Toggle
          checked={settings.aAndYAsVariables}
          label="a und y als Variablen"
          hint="sonst die Einheiten Jahr (a, y)"
          onChange={(v) => set('aAndYAsVariables', v)}
        />
        <Toggle
          checked={settings.letterProducts}
          label="Buchstabenfolgen als Produkt"
          hint="2ab wird als 2 · a · b gelesen"
          onChange={(v) => set('letterProducts', v)}
        />
      </section>

      {props.error && <p className="setting-error">{props.error}</p>}

      <button className="setting-reset" onClick={() => onChange(DEFAULT_SETTINGS)}>
        Standard wiederherstellen
      </button>
    </div>
  );
}
