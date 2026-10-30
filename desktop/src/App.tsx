import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { CSSProperties } from 'react';
import { api, ApiError, type Calculation, type Mode, type Settings, type StepKind, type Variable } from './api';
import { Katex, latexLines } from './components/Katex';
import { LatexInput, type LatexInputHandle } from './components/LatexInput';
import { ElectronicsView } from './components/ElectronicsView';
import { AnalysisView } from './components/AnalysisView';
import { LinearSystemView } from './components/LinearSystemView';
import { EMPTY_LINEAR, type LinearInput } from './linear';
import { fromFile, SHOWN, toFile, type EditorElement, type Shown } from './circuit';
import { open, save } from '@tauri-apps/plugin-dialog';
import { copyText, inShell, onShortcut } from './shell';
import { MathField, toReadableLatex, type MathFieldHandle } from './components/MathField';
import { I18nContext, makeTranslate } from './i18n';
import type { TextKey } from './i18n/en';
import { DEFAULT_SETTINGS, modeChoices, Segmented, SettingsPanel } from './components/SettingsPanel';

interface HistoryEntry {
  id: number;
  /** The canonical input, as the backend re-rendered it. */
  latex: string;
  result: string;
  at: Date;
}

type Panel = 'history' | 'variables' | 'settings' | null;

const STEP_LABELS: Record<StepKind, TextKey> = {
  symbolic: 'step.symbolic',
  substituted: 'step.substituted',
  result: 'step.result',
  algebraic: 'step.algebraic',
  note: 'note',
};

/** An example under the empty field that fits what the mode reads; analysis and electronics bring their own. */
const PLACEHOLDERS: Partial<Record<Mode, string>> = {
  algebra: '\\frac{x^2 - 1}{x + 1}',
  physics: 'F = 2kg \\cdot 10\\frac{m}{s^2}',
  imaginary: '(2 + 3i)(1 - i)',
};

export function App() {
  const [input, setInput] = useState('');
  const [calculation, setCalculation] = useState<Calculation | null>(null);
  const [error, setError] = useState<ApiError | null>(null);
  const [variables, setVariables] = useState<Variable[]>([]);
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [stepsOpen, setStepsOpen] = useState(true);
  const [panel, setPanel] = useState<Panel>(null);
  const [precision, setPrecision] = useState<number | null>(null);
  const [copied, setCopied] = useState<string | null>(null);
  const [settings, setSettings] = useState<Settings>(DEFAULT_SETTINGS);
  const [settingsLoaded, setSettingsLoaded] = useState(false);
  const [settingsError, setSettingsError] = useState<string | null>(null);
  /** Quelltext statt Formeleditor — über `\` oder Strg+L (F2). */
  const [latexMode, setLatexMode] = useState(false);
  /** The schematic lives here so it survives a switch to another mode and back. */
  const [circuit, setCircuit] = useState<EditorElement[]>([]);
  /** Kept here rather than in the editor, so it outlasts a switch to another mode. */
  const [circuitShown, setCircuitShown] = useState<Shown>(SHOWN);
  /** The linear system and its tab, kept here for the same reason. */
  const [linear, setLinear] = useState<LinearInput>(EMPTY_LINEAR);
  const [fileError, setFileError] = useState<string | null>(null);

  const language = settings.language;
  const placeholder = PLACEHOLDERS[settings.mode] ?? PLACEHOLDERS.physics;
  // At the detail level `none` the engine still computes the steps; only the list is hidden.
  const showSteps = settings.stepDetail !== 'none';
  const i18n = useMemo(() => ({ language, t: makeTranslate(language) }), [language]);
  useEffect(() => {
    document.documentElement.lang = language;
  }, [language]);

  const { t } = i18n;
  const ctrl = t('key.ctrl');
  const FILTERS = [{ name: t('file.filter'), extensions: ['json'] }];

  /** Replaces the circuit only when the file checks out; a broken one leaves it untouched. */
  const loadCircuit = async () => {
    try {
      const path = await open({ filters: FILTERS, multiple: false });
      if (typeof path !== 'string') return;
      setCircuit(fromFile(await api.loadCircuit(path)));
      setFileError(null);
    } catch (failure) {
      setFileError(failure instanceof Error ? failure.message : String(failure));
    }
  };

  const saveCircuit = async () => {
    try {
      const path = await save({ filters: FILTERS, defaultPath: t('file.default') });
      if (!path) return;
      await api.saveCircuit(path, toFile(circuit));
      setFileError(null);
    } catch (failure) {
      setFileError(failure instanceof Error ? failure.message : String(failure));
    }
  };

  const field = useRef<MathFieldHandle>(null);
  const latexField = useRef<LatexInputHandle>(null);
  const editor = () => (latexMode ? latexField.current : field.current);
  // The menu shortcuts fire from outside React's render cycle and must not see stale state.
  const latest = useRef({ input, calculation, error });
  latest.current = { input, calculation, error };

  const refreshVariables = useCallback(async () => {
    try {
      setVariables(await api.variables());
    } catch {
      /* the engine lives in the app; a failure here leaves the list as it was */
    }
  }, []);

  /** `record: false` rechnet nur neu (nach geänderten Einstellungen), ohne Verlaufseintrag. */
  const evaluate = useCallback(async ({ record = true } = {}) => {
    const latex = latest.current.input.trim();
    if (!latex) return;
    try {
      const result = await api.evaluate(latex);
      setCalculation(result);
      setError(null);
      if (record) {
        setHistory((entries) =>
          [{ id: Date.now(), latex: result.input, result: result.result, at: new Date() }, ...entries].slice(0, 50),
        );
      }
      if (result.variable) await refreshVariables();
    } catch (failure) {
      setCalculation(null);
      setError(failure instanceof ApiError ? failure : new ApiError(String(failure)));
    }
  }, [refreshVariables]);

  const copy = useCallback(async (text: string, marker: string) => {
    await copyText(text);
    setCopied(marker);
    window.setTimeout(() => setCopied((current) => (current === marker ? null : current)), 1200);
  }, []);

  const reset = useCallback(() => {
    setInput('');
    setCalculation(null);
    setError(null);
    field.current?.setValue('');
    latexField.current?.focus();
  }, []);

  const togglePanel = useCallback((next: Exclude<Panel, null>) => {
    setPanel((current) => (current === next ? null : next));
  }, []);

  /** Übernimmt eine Formel ins gerade sichtbare Eingabefeld — Verlauf, Kürzel, Panels. */
  const load = useCallback((latex: string) => {
    setInput(latex);
    field.current?.setValue(latex);
  }, []);

  /**
   * Wechsel zwischen Formeleditor und Quelltext. Der Inhalt geht dabei nicht verloren: In den
   * Quelltext kommt er in der Schreibweise, in der man ihn auch tippen würde, zurück im
   * Formeleditor liest MathLive ihn wieder als Formel ein.
   */
  const toggleLatexMode = useCallback((seed?: string) => {
    setLatexMode((current) => {
      if (!current) setInput((text) => seed ?? toReadableLatex(text));
      return !current;
    });
  }, []);

  // The engine runs inside the app, so there is nothing to wait for.
  useEffect(() => {
    api.info().then((info) => setPrecision(info.precision), () => {});
    api.settings().then(setSettings, () => {}).finally(() => setSettingsLoaded(true));
    void refreshVariables();
  }, [refreshVariables]);

  /** Speichert sofort und rechnet die angezeigte Eingabe neu, damit man die Wirkung sieht. */
  const changeSettings = useCallback(
    async (next: Settings) => {
      try {
        const saved = await api.saveSettings(next);
        setSettings(saved);
        setPrecision(saved.precision);
        setSettingsError(null);
        // The analysis and linear systems views re-run themselves; the hidden calculator input need not.
        if (saved.mode !== 'analysis' && saved.mode !== 'linear' && (latest.current.calculation || latest.current.error)) await evaluate({ record: false });
      } catch (failure) {
        setSettingsError(failure instanceof Error ? failure.message : String(failure));
      }
    },
    [evaluate],
  );

  // The menu accelerators of the native shell (F10).
  useEffect(() => {
    return onShortcut((shortcut) => {
      const current = latest.current.calculation;
      switch (shortcut) {
        case 'new':
          reset();
          break;
        case 'evaluate':
          void evaluate();
          break;
        case 'toggle-steps':
          setStepsOpen((open) => !open);
          break;
        case 'copy-result':
          if (current) void copy(current.latex, 'result');
          break;
        case 'copy-all':
          if (current) void copy(current.alignBlock, 'all');
          break;
        case 'toggle-variables':
          togglePanel('variables');
          break;
        case 'toggle-history':
          togglePanel('history');
          break;
        case 'toggle-latex':
          toggleLatexMode();
          break;
        case 'toggle-settings':
          togglePanel('settings');
          break;
      }
    });
  }, [copy, evaluate, reset, togglePanel, toggleLatexMode]);

  // Escape closes the panel everywhere; the Ctrl combos only stand in for the menu when the
  // UI runs in a plain browser (`npm run dev:renderer`).
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setPanel(null);
        editor()?.focus();
        return;
      }
      if (inShell || !event.ctrlKey) return;
      const key = event.key.toLowerCase();
      if (key === 'l') {
        event.preventDefault();
        toggleLatexMode();
      } else if (key === 'j') {
        event.preventDefault();
        setStepsOpen((open) => !open);
      } else if (key === 'h') {
        event.preventDefault();
        togglePanel('history');
      } else if (key === ',') {
        event.preventDefault();
        togglePanel('variables');
      } else if (key === '.') {
        event.preventDefault();
        togglePanel('settings');
      }
    };
    document.addEventListener('keydown', onKeyDown);
    return () => document.removeEventListener('keydown', onKeyDown);
    // `editor()` liest nur Refs und braucht deshalb nicht in die Abhängigkeiten.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [togglePanel, toggleLatexMode]);

  const forget = async (key: string) => {
    await api.forget(key);
    await refreshVariables();
  };

  if (!settingsLoaded) return null;

  return (
    <I18nContext.Provider value={i18n}>
    <div className={panel ? 'app panel-open' : 'app'}>
      <header className="topbar">
        <div className="wordmark">
          QUANTU<span>S</span>
        </div>
        <div className="topbar-mid">
          <div className="dim-readout">
            <span className="bra">⟨</span>
            <span className="vec">{calculation?.dimension ?? '—'}</span>
            <span className="bra">⟩</span>
            <span className="arrow">→</span>
            <span className="unit">{calculation ? calculation.unit : '·'}</span>
          </div>
        </div>
        <div className="topbar-right">
          <div className="mode-switch">
            <Segmented
              choices={modeChoices(t, settings.linearComplex)}
              value={settings.mode}
              onChange={(mode) => void changeSettings({ ...settings, mode })}
            />
          </div>
          <span className="precision-tag">{precision ? `${precision} sig` : '—'}</span>
          <span>
            <span className="status-dot" /> {t('status.ready')}
          </span>
        </div>
      </header>

      {settings.mode === 'analysis' ? (
        <AnalysisView settings={settings} />
      ) : settings.mode === 'linear' ? (
        <LinearSystemView
          settings={settings}
          onSettingsChange={(next) => void changeSettings(next)}
          input={linear}
          onInputChange={setLinear}
        />
      ) : settings.mode === 'electronics' ? (
        <main className="workspace circuit-workspace">
          <ElectronicsView
            elements={circuit}
            onChange={setCircuit}
            settings={settings}
            shown={circuitShown}
            onShownChange={setCircuitShown}
            toolbar={
              <div className="file-buttons">
                <div className="palette-title">{t('file.title')}</div>
                <button className="palette-item" onClick={() => void loadCircuit()}>
                  {t('file.load')}
                </button>
                <button className="palette-item" onClick={() => void saveCircuit()}>
                  {t('file.save')}
                </button>
                {fileError && <p className="file-error">{fileError}</p>}
              </div>
            }
          />
        </main>
      ) : (
      <main className="workspace">
        <div className="calc-panel">
          <div className={latexMode ? 'input-row latex' : 'input-row'}>
            {latexMode ? (
              <LatexInput
                ref={latexField}
                value={input}
                onChange={setInput}
                onEnter={() => void evaluate()}
                onLeave={() => toggleLatexMode()}
                placeholder={placeholder}
              />
            ) : (
              <MathField
                ref={field}
                value={input}
                onChange={setInput}
                onEnter={() => void evaluate()}
                onLatexRequested={(seed) => toggleLatexMode(seed)}
              />
            )}
            <button
              className="mode-toggle"
              title={latexMode ? t('editor.back') : t('editor.latex', { ctrl })}
              onClick={() => toggleLatexMode()}
            >
              {latexMode ? 'TeX' : 'f(x)'}
            </button>
          </div>

          {/*
            Unter dem Feld steht immer die jeweils andere Sicht: beim Tippen im Formeleditor
            der Quelltext zum Kopieren, beim Schreiben von LaTeX die gesetzte Formel — damit
            man sieht, ob der Ausdruck das ist, was man gemeint hat.
          */}
          <div className="source-row" style={{ '--lines': latexMode && input ? latexLines(input) : 1 } as CSSProperties}>
            {latexMode ? (
              input ? (
                <Katex className="source-preview" latex={`\\displaystyle ${input}`} />
              ) : (
                <code className="source">{placeholder}</code>
              )
            ) : (
              <code className="source">{input || placeholder}</code>
            )}
            {input && (
              <button className="copy-btn" onClick={() => void copy(input, 'input')}>
                {copied === 'input' ? t('copied') : t('copy')}
              </button>
            )}
          </div>

          {error ? (
            <div className="error-row">
              <span className="error-label">{t('error')}</span>
              <span className="error-text">
                {error.message}
                {error.position !== null && <span className="error-position"> · {t('position', { n: error.position + 1 })}</span>}
              </span>
            </div>
          ) : (
            <div className="result-container">
              <div className="result-row">
                <div className="result-main">
                  <span className="result-eq">=</span>
                  {calculation ? (
                    <Katex className="result-value" latex={calculation.result} />
                  ) : (
                    <span className="result-value">–</span>
                  )}
                  {calculation?.quantityName && <span className="result-unit">{calculation.quantityName}</span>}
                </div>
                {calculation && showSteps && (
                  <button
                    className={stepsOpen ? 'steps-toggle open' : 'steps-toggle'}
                    onClick={() => setStepsOpen((open) => !open)}
                  >
                    {t('steps')} <span className="chev">⌄</span> <kbd>{ctrl}+J</kbd>
                  </button>
                )}
              </div>
              {calculation?.warning && (
                <div className="domain-row warning-row">
                  <span className="domain-label">{t('note')}</span>
                  <span>{calculation.warning}</span>
                </div>
              )}
              {calculation?.domainRestrictions && calculation.domainRestrictions.length > 0 && (
                <div className="domain-row">
                  <span className="domain-label">{t('domain')}</span>
                  <div className="domain-list">
                    {calculation.domainRestrictions.map((r, i) => (
                      <span className="domain-item" key={i}>
                        {t('domain.for')} <Katex latex={r} />
                      </span>
                    ))}
                  </div>
                </div>
              )}
            </div>
          )}

          {calculation && showSteps && stepsOpen && (
            <div className="steps-panel">
              {calculation.steps.map((step, index) => (
                <div
                  className={`step-row ${step.explanation ? 'step-row-with-explanation' : ''}`}
                  key={`${step.kind}-${index}`}
                >
                  {step.explanation ? (
                    <>
                      <div className="step-header">
                        <span className={step.kind === 'result' ? 'step-label result-label' : 'step-label'}>
                          {t(STEP_LABELS[step.kind])}
                        </span>
                        <span className="step-explanation">{step.explanation}</span>
                      </div>
                      <div className="step-body">
                        <Katex className="step-latex" latex={step.latex} />
                        <button className="copy-btn" onClick={() => void copy(step.latex, `step-${index}`)}>
                          {copied === `step-${index}` ? t('copied') : t('copy')}
                        </button>
                      </div>
                    </>
                  ) : (
                    <>
                      <span className={step.kind === 'result' ? 'step-label result-label' : 'step-label'}>
                        {t(STEP_LABELS[step.kind])}
                      </span>
                      <Katex className="step-latex" latex={step.latex} />
                      <button className="copy-btn" onClick={() => void copy(step.latex, `step-${index}`)}>
                        {copied === `step-${index}` ? t('copied') : t('copy')}
                      </button>
                    </>
                  )}
                </div>
              ))}
              <div className="step-row step-row-all">
                <span className="step-label">{t('total')}</span>
                <span className="step-hint">{t('total.hint', { ctrl })}</span>
                <button className="copy-btn" onClick={() => void copy(calculation.alignBlock, 'all')}>
                  {copied === 'all' ? t('copied') : t('copy')}
                </button>
              </div>
            </div>
          )}
        </div>
      </main>
      )}

      <div className="side-rail">
        <button
          className={panel === 'history' ? 'rail-btn active' : 'rail-btn'}
          title={`${t('history')} (${ctrl}+H)`}
          onClick={() => togglePanel('history')}
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6">
            <circle cx="12" cy="12" r="8.5" />
            <path d="M12 7.5V12l3 2" />
          </svg>
        </button>
        <button
          className={panel === 'variables' ? 'rail-btn active' : 'rail-btn'}
          title={`${t('variables')} (${ctrl}+,)`}
          onClick={() => togglePanel('variables')}
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6">
            <path d="M9 5c-2 0-2.5 1-2.5 3s0 3-2 4c2 1 2 2 2 4s.5 3 2.5 3" />
            <path d="M15 5c2 0 2.5 1 2.5 3s0 3 2 4c-2 1-2 2-2 4s-.5 3-2.5 3" />
          </svg>
        </button>
        <button
          className={panel === 'settings' ? 'rail-btn active' : 'rail-btn'}
          title={`${t('settings')} (${ctrl}+.)`}
          onClick={() => togglePanel('settings')}
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6">
            <path d="M4 7h10M18 7h2M4 17h4M12 17h8" />
            <circle cx="16" cy="7" r="2" />
            <circle cx="10" cy="17" r="2" />
          </svg>
        </button>
      </div>

      <aside className={panel ? 'side-panel' : 'side-panel hidden'}>
        <div className="panel-tabs">
          <button
            className={panel === 'history' ? 'panel-tab active' : 'panel-tab'}
            onClick={() => setPanel('history')}
          >
            {t('history')}
          </button>
          <button
            className={panel === 'variables' ? 'panel-tab active' : 'panel-tab'}
            onClick={() => setPanel('variables')}
          >
            {t('variables')}
          </button>
          <button
            className={panel === 'settings' ? 'panel-tab active' : 'panel-tab'}
            onClick={() => setPanel('settings')}
          >
            {t('settings')}
          </button>
          {/* Das Panel liegt über der Leiste, deren Knöpfe es sonst schließen würden. */}
          <button
            className="panel-collapse"
            title={t('panel.collapse')}
            onClick={() => {
              setPanel(null);
              editor()?.focus();
            }}
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6">
              <path d="M7 6l6 6-6 6M13 6l6 6-6 6" />
            </svg>
          </button>
        </div>
        <div className="panel-body">
          {panel === 'history' &&
            (history.length === 0 ? (
              <p className="panel-empty">{t('history.empty')}</p>
            ) : (
              history.map((entry) => (
                <div className="hist-item" key={entry.id} onClick={() => load(entry.latex)}>
                  <Katex className="hist-latex" latex={entry.latex} />
                  <div className="hist-meta">
                    <span>{entry.at.toLocaleTimeString(language, { hour: '2-digit', minute: '2-digit' })}</span>
                    <Katex className="val" latex={entry.result} />
                  </div>
                </div>
              ))
            ))}

          {panel === 'variables' &&
            (variables.length === 0 ? (
              <p className="panel-empty">
                {t('variables.empty')} <code>r_1 = 0{language === 'de' ? ',' : '.'}5m</code>.
              </p>
            ) : (
              variables.map((variable) => (
                <div className="var-item" key={variable.key}>
                  <button className="var-insert" onClick={() => editor()?.insert(variable.symbol)}>
                    <Katex className="var-name" latex={variable.symbol} />
                    <Katex className="var-val" latex={variable.latex} />
                  </button>
                  <button className="var-forget" title={t('variable.forget')} onClick={() => void forget(variable.key)}>
                    ✕
                  </button>
                </div>
              ))
            ))}

          {panel === 'settings' && (
            <SettingsPanel settings={settings} error={settingsError} onChange={(next) => void changeSettings(next)} />
          )}
        </div>
      </aside>

      <footer className="statusbar">
        <span>{t('footer.session')}</span>
        <span>
          {t('footer.counts', { variables: variables.length, history: history.length })}
        </span>
      </footer>
    </div>
    </I18nContext.Provider>
  );
}
