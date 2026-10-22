import { useCallback, useEffect, useRef, useState } from 'react';
import { api, ApiError, type Calculation, type StepKind, type Variable } from './api';
import { Katex } from './components/Katex';
import { LatexInput, type LatexInputHandle } from './components/LatexInput';
import { copyText, inShell, onShortcut } from './shell';
import { MathField, toReadableLatex, type MathFieldHandle } from './components/MathField';

interface HistoryEntry {
  id: number;
  /** The canonical input, as the backend re-rendered it. */
  latex: string;
  result: string;
  at: Date;
}

type Panel = 'history' | 'variables' | null;

const STEP_LABELS: Record<StepKind, string> = {
  symbolic: 'SYMBOLISCH',
  substituted: 'EINGESETZT',
  result: 'ERGEBNIS',
  algebraic: 'SCHRITT',
};

const PLACEHOLDER = 'F = 2kg \\cdot 10\\frac{m}{s^2}';

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
  /** Quelltext statt Formeleditor — über `\` oder Strg+L (F2). */
  const [latexMode, setLatexMode] = useState(false);

  const field = useRef<MathFieldHandle>(null);
  const latexField = useRef<LatexInputHandle>(null);
  const editor = () => (latexMode ? latexField.current : field.current);
  // The menu shortcuts fire from outside React's render cycle and must not see stale state.
  const latest = useRef({ input, calculation });
  latest.current = { input, calculation };

  const refreshVariables = useCallback(async () => {
    try {
      setVariables(await api.variables());
    } catch {
      /* the engine lives in the app; a failure here leaves the list as it was */
    }
  }, []);

  const evaluate = useCallback(async () => {
    const latex = latest.current.input.trim();
    if (!latex) return;
    try {
      const result = await api.evaluate(latex);
      setCalculation(result);
      setError(null);
      setHistory((entries) =>
        [{ id: Date.now(), latex: result.input, result: result.result, at: new Date() }, ...entries].slice(0, 50),
      );
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
    void refreshVariables();
  }, [refreshVariables]);

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

  return (
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
          <span className="precision-tag">{precision ? `${precision} sig` : '—'}</span>
          <span>
            <span className="status-dot" /> bereit
          </span>
        </div>
      </header>

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
              title={latexMode ? 'Zurück zum Formeleditor (Esc)' : 'LaTeX schreiben (\\ oder Strg+L)'}
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
          <div className="source-row">
            {latexMode ? (
              input ? (
                <Katex className="source-preview" latex={input} />
              ) : (
                <code className="source">{PLACEHOLDER}</code>
              )
            ) : (
              <code className="source">{input || PLACEHOLDER}</code>
            )}
            {input && (
              <button className="copy-btn" onClick={() => void copy(input, 'input')}>
                {copied === 'input' ? 'kopiert' : 'copy'}
              </button>
            )}
          </div>

          {error ? (
            <div className="error-row">
              <span className="error-label">FEHLER</span>
              <span className="error-text">
                {error.message}
                {error.position !== null && <span className="error-position"> · Position {error.position + 1}</span>}
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
                {calculation && (
                  <button
                    className={stepsOpen ? 'steps-toggle open' : 'steps-toggle'}
                    onClick={() => setStepsOpen((open) => !open)}
                  >
                    Rechenweg <span className="chev">⌄</span> <kbd>Strg+J</kbd>
                  </button>
                )}
              </div>
              {calculation?.domainRestrictions && calculation.domainRestrictions.length > 0 && (
                <div className="domain-row">
                  <span className="domain-label">DEFINITIONSBEREICH</span>
                  <div className="domain-list">
                    {calculation.domainRestrictions.map((r, i) => (
                      <span className="domain-item" key={i}>
                        für <Katex latex={r} />
                      </span>
                    ))}
                  </div>
                </div>
              )}
            </div>
          )}

          {calculation && stepsOpen && (
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
                          {STEP_LABELS[step.kind]}
                        </span>
                        <span className="step-explanation">{step.explanation}</span>
                      </div>
                      <div className="step-body">
                        <Katex className="step-latex" latex={step.latex} />
                        <button className="copy-btn" onClick={() => void copy(step.latex, `step-${index}`)}>
                          {copied === `step-${index}` ? 'kopiert' : 'copy'}
                        </button>
                      </div>
                    </>
                  ) : (
                    <>
                      <span className={step.kind === 'result' ? 'step-label result-label' : 'step-label'}>
                        {STEP_LABELS[step.kind]}
                      </span>
                      <Katex className="step-latex" latex={step.latex} />
                      <button className="copy-btn" onClick={() => void copy(step.latex, `step-${index}`)}>
                        {copied === `step-${index}` ? 'kopiert' : 'copy'}
                      </button>
                    </>
                  )}
                </div>
              ))}
              <div className="step-row step-row-all">
                <span className="step-label">GESAMT</span>
                <span className="step-hint">als align*-Block · Strg+Alt+C</span>
                <button className="copy-btn" onClick={() => void copy(calculation.alignBlock, 'all')}>
                  {copied === 'all' ? 'kopiert' : 'copy'}
                </button>
              </div>
            </div>
          )}
        </div>
      </main>

      <div className="side-rail">
        <button
          className={panel === 'history' ? 'rail-btn active' : 'rail-btn'}
          title="Verlauf (Strg+H)"
          onClick={() => togglePanel('history')}
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6">
            <circle cx="12" cy="12" r="8.5" />
            <path d="M12 7.5V12l3 2" />
          </svg>
        </button>
        <button
          className={panel === 'variables' ? 'rail-btn active' : 'rail-btn'}
          title="Variablen (Strg+,)"
          onClick={() => togglePanel('variables')}
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6">
            <path d="M9 5c-2 0-2.5 1-2.5 3s0 3-2 4c2 1 2 2 2 4s.5 3 2.5 3" />
            <path d="M15 5c2 0 2.5 1 2.5 3s0 3 2 4c-2 1-2 2-2 4s-.5 3-2.5 3" />
          </svg>
        </button>
      </div>

      <aside className={panel ? 'side-panel' : 'side-panel hidden'}>
        <div className="panel-tabs">
          <button
            className={panel === 'history' ? 'panel-tab active' : 'panel-tab'}
            onClick={() => setPanel('history')}
          >
            Verlauf
          </button>
          <button
            className={panel === 'variables' ? 'panel-tab active' : 'panel-tab'}
            onClick={() => setPanel('variables')}
          >
            Variablen
          </button>
        </div>
        <div className="panel-body">
          {panel === 'history' &&
            (history.length === 0 ? (
              <p className="panel-empty">Noch nichts gerechnet.</p>
            ) : (
              history.map((entry) => (
                <div className="hist-item" key={entry.id} onClick={() => load(entry.latex)}>
                  <Katex className="hist-latex" latex={entry.latex} />
                  <div className="hist-meta">
                    <span>{entry.at.toLocaleTimeString('de-DE', { hour: '2-digit', minute: '2-digit' })}</span>
                    <Katex className="val" latex={entry.result} />
                  </div>
                </div>
              ))
            ))}

          {panel === 'variables' &&
            (variables.length === 0 ? (
              <p className="panel-empty">
                Noch keine Variablen. Definiere eine mit <code>r_1 = 0{','}5m</code>.
              </p>
            ) : (
              variables.map((variable) => (
                <div className="var-item" key={variable.key}>
                  <button className="var-insert" onClick={() => editor()?.insert(variable.symbol)}>
                    <Katex className="var-name" latex={variable.symbol} />
                    <Katex className="var-val" latex={variable.latex} />
                  </button>
                  <button className="var-forget" title="Variable löschen" onClick={() => void forget(variable.key)}>
                    ✕
                  </button>
                </div>
              ))
            ))}
        </div>
      </aside>

      <footer className="statusbar">
        <span>Symja · Indriya — Session aktiv</span>
        <span>
          {variables.length} Variablen · {history.length} Einträge im Verlauf
        </span>
      </footer>
    </div>
  );
}
