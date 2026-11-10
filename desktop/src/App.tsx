import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { CSSProperties } from 'react';
import { api, ApiError, setSessionId, type Calculation, type FormKind, type HistoryEntry, type ModeInputs, type Mode, type NumberBase, type SessionInfo, type Settings, type StepKind, type Variable } from './api';
import { Katex, latexLines } from './components/Katex';
import { LatexInput, type LatexInputHandle } from './components/LatexInput';
import { ElectronicsView } from './components/ElectronicsView';
import { KirchhoffView } from './components/KirchhoffView';
import { AnalysisView } from './components/AnalysisView';
import { LinearSystemView } from './components/LinearSystemView';
import { GeometryView } from './components/GeometryView';
import { EMPTY_GEOMETRY, geometryInput, geometryRequest, type GeometryInput } from './geometry';
import { EMPTY_LINEAR, inputFor, requestFor, type LinearInput } from './linear';
import { fromFile, SHOWN, toFile, type EditorElement, type Shown } from './circuit';
import { open, save } from '@tauri-apps/plugin-dialog';
import { copyText, inShell, onShortcut, type Shortcut } from './shell';
import { MathField, toReadableLatex, type MathFieldHandle, type Tool } from './components/MathField';
import { InsertBar, type InsertKey } from './components/InsertBar';
import { LibraryDialog } from './components/LibraryDialog';
import { WriteupDialog } from './components/WriteupDialog';
import { QuickSearch } from './components/QuickSearch';
import { SessionsPanel } from './components/SessionsPanel';
import { ToolDialog, type ToolKind } from './components/ToolDialog';
import { I18nContext, makeTranslate } from './i18n';
import type { TextKey } from './i18n';
import { DEFAULT_SETTINGS, modeChoices, Segmented, SettingsDialog } from './components/SettingsDialog';
import { useTheme } from './theme';

type Panel = 'history' | 'variables' | 'sessions' | null;

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
  baseN: '1010 \\operatorname{and} 0110',
  verify: '2^{10} > 10^{3}',
  geometry: 'A(1|2|3)',
};

/** The modes that know units and natural constants, as `Settings::parse_options` has them. */
const UNIT_MODES: Mode[] = ['physics', 'electronics', 'kirchhoff'];

const BASE_CHOICES: { value: NumberBase; label: string }[] = [
  { value: 'dec', label: 'DEC' },
  { value: 'hex', label: 'HEX' },
  { value: 'bin', label: 'BIN' },
  { value: 'oct', label: 'OCT' },
];

/** The logic of Basis-N and the prefixes that read one number in another base. */
const BASE_KEYS: InsertKey[] = [
  ...['and', 'or', 'xor', 'xnor'].map((word) => ({ label: word, latex: ` \\operatorname{${word}} ` })),
  ...['Not', 'Neg'].map((word) => ({ label: `${word}(`, latex: `\\operatorname{${word}}\\left(#?\\right)` })),
  ...['d', 'h', 'b', 'o'].map((prefix) => ({ label: prefix, latex: `\\mathrm{${prefix}}` })),
];

/** The relations Prüfen checks, chained as on a Casio: `1 < 2 < 3`. */
const VERIFY_KEYS: InsertKey[] = ['=', '\\ne', '<', '>', '\\le', '\\ge'].map((latex) => ({ label: latex, latex, math: true }));

/**
 * The forms the result can be switched between: its `forms`, and the result as the engine wrote
 * it when that is none of them (a decimal whose fractions are the forms).
 */
function formChoices(calculation: Calculation | null): { kind: FormKind | 'result'; latex: string }[] {
  if (!calculation?.forms?.length) return [];
  const { forms, result } = calculation;
  return forms.some((form) => form.latex === result) ? forms : [{ kind: 'result', latex: result }, ...forms];
}

/** Whether ENG has a number to work on: a decimal value, not a Basis-N or true/false result. */
const engineeringApplies = (calculation: Calculation, mode: Mode) =>
  mode !== 'baseN' && mode !== 'verify' && calculation.value !== '' && !Number.isNaN(Number(calculation.value));

export function App() {
  const [input, setInput] = useState('');
  const [calculation, setCalculation] = useState<Calculation | null>(null);
  const [error, setError] = useState<ApiError | null>(null);
  const [variables, setVariables] = useState<Variable[]>([]);
  /** The open session's history, oldest first, as the engine keeps it. */
  const [history, setHistory] = useState<HistoryEntry[]>([]);
  const [sessions, setSessions] = useState<SessionInfo[]>([]);
  const [sessionId, setCurrentSession] = useState('');
  /** Which of `formChoices` is shown; null is the result as the engine wrote it. */
  const [formIndex, setFormIndex] = useState<number | null>(null);
  /** The result after ENG or ←ENG, with the steps taken so far. */
  const [eng, setEng] = useState<{ steps: number; latex: string } | null>(null);
  const [tool, setTool] = useState<ToolKind | null>(null);
  const [stepsOpen, setStepsOpen] = useState(true);
  const [panel, setPanel] = useState<Panel>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [libraryOpen, setLibraryOpen] = useState(false);
  const [writeupOpen, setWriteupOpen] = useState(false);
  const [quickOpen, setQuickOpen] = useState(false);
  const [precision, setPrecision] = useState<number | null>(null);
  const [copied, setCopied] = useState<string | null>(null);
  const [settings, setSettings] = useState<Settings>(DEFAULT_SETTINGS);
  const [settingsLoaded, setSettingsLoaded] = useState(false);
  const [settingsError, setSettingsError] = useState<string | null>(null);
  // Waits for the saved settings, so a dark theme does not flash light first on a light system.
  const theme = useTheme(settingsLoaded ? settings.theme : null);
  /** Quelltext statt Formeleditor — über `\` oder Strg+L (F2). */
  const [latexMode, setLatexMode] = useState(false);
  /** The schematic lives here so it survives a switch to another mode and back. */
  const [circuit, setCircuit] = useState<EditorElement[]>([]);
  /** The Kirchhoff mode draws its own network; files are shared with the electronics mode. */
  const [kirchhoffCircuit, setKirchhoffCircuit] = useState<EditorElement[]>([]);
  /** Kept here rather than in the editor, so it outlasts a switch to another mode. */
  const [circuitShown, setCircuitShown] = useState<Shown>(SHOWN);
  /** The linear system and its tab, kept here for the same reason. */
  const [linear, setLinear] = useState<LinearInput>(EMPTY_LINEAR);
  /** The Geometry scene and its task, kept here for the same reason. */
  const [geometry, setGeometry] = useState<GeometryInput>(EMPTY_GEOMETRY);
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

  /** The circuit of the mode that is open: the Kirchhoff mode has its own. */
  const kirchhoff = settings.mode === 'kirchhoff';

  /** Replaces the circuit only when the file checks out; a broken one leaves it untouched. */
  const loadCircuit = async () => {
    try {
      const path = await open({ filters: FILTERS, multiple: false });
      if (typeof path !== 'string') return;
      (kirchhoff ? setKirchhoffCircuit : setCircuit)(fromFile(await api.loadCircuit(path)));
      setFileError(null);
    } catch (failure) {
      setFileError(failure instanceof Error ? failure.message : String(failure));
    }
  };

  const saveCircuit = async () => {
    try {
      const path = await save({ filters: FILTERS, defaultPath: t('file.default') });
      if (!path) return;
      await api.saveCircuit(path, toFile(kirchhoff ? kirchhoffCircuit : circuit));
      setFileError(null);
    } catch (failure) {
      setFileError(failure instanceof Error ? failure.message : String(failure));
    }
  };

  const fileButtons = (
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
  );

  const field = useRef<MathFieldHandle>(null);
  const latexField = useRef<LatexInputHandle>(null);
  const editor = () => (latexMode ? latexField.current : field.current);
  // The menu shortcuts fire from outside React's render cycle and must not see stale state.
  const latest = useRef({ input, calculation, error, formIndex, eng, mode: settings.mode });
  latest.current = { input, calculation, error, formIndex, eng, mode: settings.mode };
  /** What the open session keeps of the modes; the linear system is written back into it. */
  const modeInputs = useRef<ModeInputs>({});

  const refreshVariables = useCallback(async () => {
    try {
      setVariables(await api.variables());
    } catch {
      /* the engine lives in the app; a failure here leaves the list as it was */
    }
  }, []);

  const refreshSessions = useCallback(async () => {
    try {
      setSessions(await api.sessions());
    } catch {
      /* as above */
    }
  }, []);

  /** A new result, shown in the form the engine wrote it. */
  const show = useCallback((result: Calculation | null) => {
    setCalculation(result);
    setFormIndex(null);
    setEng(null);
  }, []);

  /**
   * `record: false` rechnet nur neu (nach geänderten Einstellungen), ohne Verlaufseintrag: CALC
   * without values evaluates the same way and leaves the history alone.
   */
  const evaluate = useCallback(async ({ record = true } = {}) => {
    const latex = latest.current.input.trim();
    if (!latex) return;
    try {
      const result = record ? await api.evaluate(latex) : await api.calc(latex, []);
      show(result);
      setError(null);
      if (record) {
        setHistory(await api.history());
        void refreshSessions();
      }
      if (result.variable) await refreshVariables();
    } catch (failure) {
      show(null);
      setError(failure instanceof ApiError ? failure : new ApiError(String(failure)));
    }
  }, [refreshVariables, refreshSessions, show]);

  /** Opens a session: its history, variables and linear system replace what is shown. */
  const openSession = useCallback(
    async (id: string) => {
      setSessionId(id);
      setCurrentSession(id);
      show(null);
      setError(null);
      try {
        await api.openSession(id);
        const [entries, inputs] = await Promise.all([api.history(), api.modeInputs()]);
        setHistory(entries);
        modeInputs.current = inputs;
        setLinear(inputFor(inputs.linear));
        setGeometry(geometryInput(inputs.geometry));
      } catch {
        setHistory([]);
      }
      await Promise.all([refreshVariables(), refreshSessions()]);
    },
    [refreshVariables, refreshSessions, show],
  );

  const newSession = useCallback(async () => {
    const created = await api.newSession();
    await openSession(created.id);
  }, [openSession]);

  /** The result in the form chosen: ENG, one of its forms, or as the engine wrote it. */
  const forms = formChoices(calculation);
  const shownResult = calculation ? (eng?.latex ?? (formIndex !== null ? forms[formIndex].latex : calculation.result)) : '';
  const activeForm = eng ? -1 : (formIndex ?? forms.findIndex((form) => form.latex === calculation?.result));

  /** S⇔D: on to the next form of the result, round to the first. */
  const nextForm = useCallback(() => {
    const { calculation: current, formIndex: index, eng: engineering } = latest.current;
    const all = formChoices(current);
    if (!current || all.length === 0) return;
    const at = engineering ? -1 : (index ?? all.findIndex((form) => form.latex === current.result));
    setEng(null);
    setFormIndex((at + 1) % all.length);
  }, []);

  /** ENG (+1) or ←ENG (−1): the first press writes the result in engineering notation. */
  const engineering = useCallback(async (direction: 1 | -1) => {
    const { calculation: current, eng: before, mode } = latest.current;
    if (!current || !engineeringApplies(current, mode)) return;
    const steps = before === null ? (direction === 1 ? 0 : -1) : before.steps + direction;
    try {
      const { latex } = await api.engineering(current, steps);
      setEng({ steps, latex });
    } catch {
      /* a value the engine cannot step stays as it is */
    }
  }, []);

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

  // The engine runs inside the app, so there is nothing to wait for. The session used last opens
  // again; on the first start there is none, and a new one is made.
  useEffect(() => {
    api.info().then((info) => setPrecision(info.precision), () => {});
    api.settings().then(setSettings, () => {}).finally(() => setSettingsLoaded(true));
    const start = async () => {
      const saved = await api.sessions();
      if (saved.length) await openSession(saved[0].id);
      else await newSession();
    };
    start().catch(() => {});
  }, [openSession, newSession]);

  // The linear system and the Geometry scene go into the session, a moment after the last change.
  useEffect(() => {
    if (!sessionId) return;
    const timer = window.setTimeout(() => {
      const next = { ...modeInputs.current, linear: requestFor(linear), geometry: geometryRequest(geometry) };
      const same = (key: 'linear' | 'geometry') => JSON.stringify(next[key] ?? null) === JSON.stringify(modeInputs.current[key] ?? null);
      if (same('linear') && same('geometry')) return;
      modeInputs.current = next;
      api.saveModeInputs(modeInputs.current).catch(() => {});
    }, 500);
    return () => window.clearTimeout(timer);
  }, [linear, geometry, sessionId]);

  /** CALC or SOLVE on what the input holds. */
  const openTool = useCallback((kind: ToolKind) => {
    if (latest.current.input.trim()) setTool(kind);
  }, []);

  const tools = useMemo<Tool[]>(
    () => [
      { label: t('tool.calc.menu'), run: () => openTool('calc') },
      { label: t('tool.solve.menu'), run: () => openTool('solve') },
    ],
    [t, openTool],
  );

  /** Speichert sofort und rechnet die angezeigte Eingabe neu, damit man die Wirkung sieht. */
  const changeSettings = useCallback(
    async (next: Settings) => {
      try {
        const saved = await api.saveSettings(next);
        setSettings(saved);
        setPrecision(saved.precision);
        setSettingsError(null);
        // The analysis, linear systems and Geometry views re-run themselves; the hidden calculator input need not.
        if (!(['analysis', 'linear', 'geometry'] as Mode[]).includes(saved.mode) && (latest.current.calculation || latest.current.error)) await evaluate({ record: false });
      } catch (failure) {
        setSettingsError(failure instanceof Error ? failure.message : String(failure));
      }
    },
    [evaluate],
  );

  /** What the menu accelerators of the native shell (F10) and the action bar of the phone layout do. */
  const runShortcut = useCallback(
    (shortcut: Shortcut) => {
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
        case 'copy-result': {
          // A form or ENG chosen on the result is what is copied; otherwise the whole line.
          const { formIndex: index, eng: engineered } = latest.current;
          if (current) void copy(engineered?.latex ?? (index !== null ? formChoices(current)[index].latex : current.latex), 'result');
          break;
        }
        case 'calc':
          openTool('calc');
          break;
        case 'solve':
          openTool('solve');
          break;
        case 'switch-form':
          nextForm();
          break;
        case 'eng':
          void engineering(1);
          break;
        case 'eng-back':
          void engineering(-1);
          break;
        case 'new-session':
          void newSession();
          break;
        case 'toggle-sessions':
          togglePanel('sessions');
          break;
        case 'toggle-library':
          setLibraryOpen((open) => !open);
          break;
        case 'toggle-writeup':
          setWriteupOpen((open) => !open);
          break;
        case 'quick-search':
          setQuickOpen((open) => !open);
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
          setSettingsOpen((open) => !open);
          break;
      }
    },
    [copy, evaluate, reset, togglePanel, toggleLatexMode, openTool, nextForm, engineering, newSession],
  );

  useEffect(() => onShortcut(runShortcut), [runShortcut]);

  // Escape closes the panel and the settings everywhere; the Ctrl combos only stand in for the menu when the
  // UI runs in a plain browser (`npm run dev:renderer`).
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        setPanel(null);
        setSettingsOpen(false);
        setLibraryOpen(false);
        setWriteupOpen(false);
        setQuickOpen(false);
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
        setSettingsOpen((open) => !open);
      } else if (key === 'k') {
        event.preventDefault();
        setLibraryOpen((open) => !open);
      } else if (key === 'g') {
        event.preventDefault();
        setWriteupOpen((open) => !open);
      } else if (key === 'f') {
        event.preventDefault();
        setQuickOpen((open) => !open);
      } else if (key === 'o') {
        event.preventDefault();
        togglePanel('sessions');
      } else if (key === 'd') {
        event.preventDefault();
        nextForm();
      }
    };
    document.addEventListener('keydown', onKeyDown);
    return () => document.removeEventListener('keydown', onKeyDown);
    // `editor()` liest nur Refs und braucht deshalb nicht in die Abhängigkeiten.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [togglePanel, toggleLatexMode, nextForm]);

  const forget = async (key: string) => {
    await api.forget(key);
    await refreshVariables();
  };

  const clearHistory = async () => {
    await api.clearHistory();
    setHistory([]);
    void refreshSessions();
  };

  const renameSession = async (id: string, name: string) => {
    await api.renameSession(id, name).catch(() => {});
    await refreshSessions();
  };

  const duplicateSession = async (id: string) => {
    const copy = await api.duplicateSession(id);
    await openSession(copy.id);
  };

  /** Deleting the open session opens the next one, or a new one when it was the last. */
  const deleteSession = async (id: string) => {
    await api.deleteSession(id);
    const rest = await api.sessions();
    if (id !== sessionId) setSessions(rest);
    else if (rest.length) await openSession(rest[0].id);
    else await newSession();
  };

  // The source field has no placeholders to jump between, so `#?` is left out there.
  const insert = (latex: string) => editor()?.insert(latexMode ? latex.replaceAll('#?', '') : latex);
  const sessionName = sessions.find((session) => session.id === sessionId)?.name;

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
      ) : settings.mode === 'geometry' ? (
        <GeometryView settings={settings} input={geometry} onInputChange={setGeometry} placeholder={placeholder} />
      ) : settings.mode === 'electronics' ? (
        <main className="workspace circuit-workspace">
          <ElectronicsView
            elements={circuit}
            onChange={setCircuit}
            settings={settings}
            shown={circuitShown}
            onShownChange={setCircuitShown}
            toolbar={fileButtons}
          />
        </main>
      ) : kirchhoff ? (
        <main className="workspace circuit-workspace">
          <KirchhoffView
            elements={kirchhoffCircuit}
            onChange={setKirchhoffCircuit}
            settings={settings}
            shown={circuitShown}
            onShownChange={setCircuitShown}
            toolbar={fileButtons}
          />
        </main>
      ) : (
      <main className="workspace">
        <div className="calc-panel">
          {settings.mode === 'baseN' && (
            <div className="mode-tools">
              <Segmented
                choices={BASE_CHOICES}
                value={settings.numberBase}
                onChange={(numberBase) => void changeSettings({ ...settings, numberBase })}
              />
              <InsertBar keys={BASE_KEYS} onInsert={insert} />
            </div>
          )}
          {settings.mode === 'verify' && (
            <div className="mode-tools">
              <InsertBar keys={VERIFY_KEYS} onInsert={insert} />
            </div>
          )}
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
                tools={tools}
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

          {/* The menu's actions as buttons; only the phone layout shows them, it has no menu bar. */}
          <div className="action-bar">
            <button className="action-key action-evaluate" onClick={() => runShortcut('evaluate')}>
              =<span className="action-label">{t('actions.evaluate')}</span>
            </button>
            <button className="action-key" disabled={!input.trim()} onClick={() => runShortcut('calc')}>
              {t('tool.calc')}
            </button>
            <button className="action-key" disabled={!input.trim()} onClick={() => runShortcut('solve')}>
              {t('tool.solve')}
            </button>
            <button className="action-key" title={t('actions.search')} aria-label={t('actions.search')} onClick={() => runShortcut('quick-search')}>
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6">
                <circle cx="10.5" cy="10.5" r="6" />
                <path d="M15 15l5 5" />
              </svg>
            </button>
            <button className="action-key" title={t('actions.new')} aria-label={t('actions.new')} onClick={() => runShortcut('new')}>
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6">
                <path d="M12 5v14M5 12h14" />
              </svg>
            </button>
            <button className="action-key action-copy" disabled={!calculation} onClick={() => runShortcut('copy-result')}>
              {copied === 'result' ? t('copied') : t('actions.copyResult')}
            </button>
            <button className="action-key action-copy" disabled={!calculation} onClick={() => runShortcut('copy-all')}>
              {copied === 'all' ? t('copied') : t('actions.copyAll')}
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
                    <Katex className="result-value" latex={shownResult} />
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
              {calculation && (forms.length > 1 || engineeringApplies(calculation, settings.mode)) && (
                <div className="form-row">
                  {forms.length > 1 &&
                    forms.map((form, index) => (
                      <button
                        key={form.kind}
                        className={index === activeForm ? 'form-chip active' : 'form-chip'}
                        title={`${t('form.next')} (${ctrl}+D)`}
                        onClick={() => {
                          setEng(null);
                          setFormIndex(index);
                        }}
                      >
                        {form.kind === 'result' ? '=' : t(`form.${form.kind}`)}
                      </button>
                    ))}
                  {engineeringApplies(calculation, settings.mode) && (
                    <>
                      <button className={eng ? 'form-chip active' : 'form-chip'} title={`${ctrl}+E`} onClick={() => void engineering(1)}>
                        ENG
                      </button>
                      <button className="form-chip" title={`${ctrl}+${t('key.shift')}+E`} onClick={() => void engineering(-1)}>
                        ←ENG
                      </button>
                    </>
                  )}
                </div>
              )}
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
          className={libraryOpen ? 'rail-btn active' : 'rail-btn'}
          title={`${t('library')} (${ctrl}+K)`}
          onClick={() => setLibraryOpen((open) => !open)}
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6">
            <path d="M5 4.5h4v15H5zM10.5 4.5h4v15h-4z" />
            <path d="M16 5.5l3.6-1 3 14.6-3.6 1z" />
          </svg>
        </button>
        <button
          className={writeupOpen ? 'rail-btn active' : 'rail-btn'}
          title={`${t('writeup')} (${ctrl}+G)`}
          onClick={() => setWriteupOpen((open) => !open)}
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6">
            <path d="M6 3.5h9l3 3v14H6z" />
            <path d="M12 3.5v6M6 9.5h12M9 13.5h6M9 16.5h4" />
          </svg>
        </button>
        <button
          className={panel === 'sessions' ? 'rail-btn active' : 'rail-btn'}
          title={`${t('sessions')} (${ctrl}+O)`}
          onClick={() => togglePanel('sessions')}
        >
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.6">
            <path d="M4 7.5h6l2 2h8v9.5H4z" />
            <path d="M4 7.5V5h6l2 2" />
          </svg>
        </button>
        <button
          className={settingsOpen ? 'rail-btn rail-settings active' : 'rail-btn rail-settings'}
          title={`${t('settings')} (${ctrl}+.)`}
          onClick={() => setSettingsOpen((open) => !open)}
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
            className={panel === 'sessions' ? 'panel-tab active' : 'panel-tab'}
            onClick={() => setPanel('sessions')}
          >
            {t('sessions')}
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
              <>
                {history
                  .map((entry, index) => (
                    <div className="hist-item" key={index} onClick={() => load(entry.response.input)}>
                      <Katex className="hist-latex" latex={entry.response.input} />
                      <div className="hist-meta">
                        <span>
                          {entry.at !== undefined &&
                            new Date(entry.at).toLocaleTimeString(language, { hour: '2-digit', minute: '2-digit' })}
                        </span>
                        <Katex className="val" latex={entry.response.result} />
                      </div>
                    </div>
                  ))
                  .reverse()}
                <button className="setting-reset history-clear" onClick={() => void clearHistory()}>
                  {t('history.clear')}
                </button>
              </>
            ))}

          {panel === 'variables' &&
            (variables.length === 0 ? (
              <p className="panel-empty">
                {t('variables.empty')} <code>r_1 = 0{language === 'de' ? ',' : '.'}5m</code>.
              </p>
            ) : (
              variables.map((variable) => (
                <div className="var-item" key={variable.key}>
                  <button className="var-insert" onClick={() => insert(variable.symbol)}>
                    <Katex className="var-name" latex={variable.symbol} />
                    <Katex className="var-val" latex={variable.latex} />
                  </button>
                  <button className="var-forget" title={t('variable.forget')} onClick={() => void forget(variable.key)}>
                    ✕
                  </button>
                </div>
              ))
            ))}

          {panel === 'sessions' && (
            <SessionsPanel
              sessions={sessions}
              current={sessionId}
              onOpen={(id) => void openSession(id)}
              onNew={() => void newSession()}
              onRename={(id, name) => void renameSession(id, name)}
              onDuplicate={(id) => void duplicateSession(id)}
              onDelete={(id) => void deleteSession(id)}
            />
          )}
        </div>
      </aside>

      <footer className="statusbar">
        <button className="statusbar-session" title={`${t('sessions')} (${ctrl}+O)`} onClick={() => togglePanel('sessions')}>
          {sessionName ?? t('footer.session')}
        </button>
        <span>
          {t('footer.counts', { variables: variables.length, history: history.length })}
        </span>
      </footer>

      {settingsOpen && (
        <SettingsDialog
          settings={settings}
          error={settingsError}
          themeError={theme.error}
          onChange={(next) => void changeSettings(next)}
          onReloadTheme={theme.reload}
          onClose={() => {
            setSettingsOpen(false);
            editor()?.focus();
          }}
        />
      )}

      {libraryOpen && (
        <LibraryDialog
          unitsKnown={UNIT_MODES.includes(settings.mode)}
          onInsert={(latex, keepOpen) => {
            if (!keepOpen) setLibraryOpen(false);
            insert(latex);
            if (!keepOpen) editor()?.focus();
          }}
          onClose={() => {
            setLibraryOpen(false);
            editor()?.focus();
          }}
        />
      )}

      {writeupOpen && (
        <WriteupDialog
          onClose={() => {
            setWriteupOpen(false);
            editor()?.focus();
          }}
        />
      )}

      {quickOpen && (
        <QuickSearch
          onInsert={(latex) => {
            setQuickOpen(false);
            insert(latex);
            editor()?.focus();
          }}
          onClose={() => {
            setQuickOpen(false);
            editor()?.focus();
          }}
        />
      )}

      {tool && (
        <ToolDialog
          kind={tool}
          latex={input.trim()}
          onClose={() => {
            setTool(null);
            editor()?.focus();
          }}
          onDone={(result) => {
            setTool(null);
            show(result);
            setError(null);
            void refreshVariables();
            editor()?.focus();
          }}
        />
      )}
    </div>
    </I18nContext.Provider>
  );
}
