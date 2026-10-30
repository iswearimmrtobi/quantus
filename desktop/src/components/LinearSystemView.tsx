import { useCallback, useEffect, useRef, useState } from 'react';
import { api, ApiError, type LinearSystem, type LinearSystemRequest, type Settings } from '../api';
import { useT } from '../i18n';
import { copyText } from '../shell';
import type { TextKey } from '../i18n/en';
import { asError, ErrorRow, Steps } from './AnalysisView';
import { Katex } from './Katex';
import { SystemField } from './SystemField';

export type LinearTab = 'equations' | 'matrix';

/** The rows of `[A | b]` with LaTeX entries, and the names of the columns if they were given. */
export interface MatrixInput {
  unknowns: string[] | null;
  rows: string[][];
}

/** The grid of the matrix tab: n unknowns, m equations (null: as many as unknowns). */
export interface GridInput {
  n: number;
  m: number | null;
  /** Entries by row and column; missing ones are empty, which the engine reads as 0. */
  cells: string[][];
  /** Names from a loaded file, used while the number of unknowns still matches. */
  unknowns: string[] | null;
}

/** What the page holds; `App` keeps it, so it survives a switch to another mode and back. */
export interface LinearInput {
  tab: LinearTab;
  /** One equation per line, as typed. */
  lines: string[];
  grid: GridInput;
  /** A matrix loaded from JSON that is too large for the grid; it is solved instead of the grid. */
  large: MatrixInput | null;
}

export const MAX_GRID = 10;

export const EMPTY_LINEAR: LinearInput = {
  tab: 'equations',
  lines: [''],
  grid: { n: 3, m: null, cells: [], unknowns: null },
  large: null,
};

const TABS: { id: LinearTab; label: TextKey }[] = [
  { id: 'equations', label: 'linear.tab.equations' },
  { id: 'matrix', label: 'linear.tab.matrix' },
];

/** The rows of the grid as the engine takes them, every row n + 1 entries long. */
export function gridRows(grid: GridInput): string[][] {
  const m = grid.m ?? grid.n;
  return Array.from({ length: m }, (_, r) => Array.from({ length: grid.n + 1 }, (_, c) => grid.cells[r]?.[c]?.trim() ?? ''));
}

/** The system as a request, or null while there is nothing to solve. */
export function requestFor(input: LinearInput): LinearSystemRequest | null {
  if (input.tab === 'equations') {
    const lines = input.lines.map((line) => line.trim()).filter(Boolean);
    return lines.length ? { equations: `\\begin{cases} ${lines.join(' \\\\ ')} \\end{cases}` } : null;
  }
  if (input.large) return matrixRequest(input.large.rows, input.large.unknowns);
  const rows = gridRows(input.grid);
  if (rows.every((row) => row.every((cell) => !cell))) return null;
  return matrixRequest(rows, input.grid.unknowns?.length === input.grid.n ? input.grid.unknowns : null);
}

/** Without names the engine names the columns by the setting. */
function matrixRequest(rows: string[][], unknowns: string[] | null): LinearSystemRequest {
  return { matrix: unknowns ? { unknowns, rows } : { rows } };
}

interface Props {
  settings: Settings;
  onSettingsChange(settings: Settings): void;
  input: LinearInput;
  onInputChange(input: LinearInput): void;
}

/**
 * The linear systems mode: a system typed line by line or entered as a matrix, solved exactly
 * with the Gauss steps. Errors are grey, because a half-typed system fails on the way.
 */
export function LinearSystemView({ settings, onSettingsChange, input, onInputChange }: Props) {
  const { t } = useT();
  const [solution, setSolution] = useState<LinearSystem | null>(null);
  const [error, setError] = useState<ApiError | null>(null);
  const [copied, setCopied] = useState(false);
  /** The latest request; an answer to an older one is dropped. */
  const request = useRef(0);
  const latest = useRef(input);
  latest.current = input;

  const solve = useCallback(async () => {
    const id = ++request.current;
    const next = requestFor(latest.current);
    if (!next) {
      setSolution(null);
      setError(null);
      return;
    }
    try {
      const result = await api.solveLinearSystem(next);
      if (id !== request.current) return;
      setSolution(result);
      setError(null);
    } catch (failure) {
      if (id !== request.current) return;
      setSolution(null);
      setError(asError(failure));
    }
  }, []);

  // On opening the page and whenever a setting changes how the system is read or written.
  const { language, precision, stepDetail, cancelMarks, elimination, unknownNames, linearComplex } = settings;
  useEffect(() => {
    void solve();
  }, [language, precision, stepDetail, cancelMarks, elimination, unknownNames, linearComplex, solve]);

  const copyAll = async () => {
    if (!solution) return;
    await copyText(solution.alignBlock);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1200);
  };

  /** Updates `latest` at once, so a solve right after a change sees it. */
  const update = (next: LinearInput) => {
    latest.current = next;
    onInputChange(next);
  };

  const changeTab = (tab: LinearTab) => {
    if (tab === input.tab) return;
    update({ ...input, tab });
    void solve();
  };

  return (
    <main className="workspace analysis-workspace">
      <div className="analysis-panel linear-panel">
        <div className="linear-bar">
          <div className="segmented analysis-tabs">
            {TABS.map((item) => (
              <button key={item.id} className={item.id === input.tab ? 'segment active' : 'segment'} onClick={() => changeTab(item.id)}>
                {t(item.label)}
              </button>
            ))}
          </div>
          <label className="setting-toggle linear-complex" title={t('linear.complex.hint')}>
            <input
              type="checkbox"
              checked={linearComplex}
              onChange={(event) => onSettingsChange({ ...settings, linearComplex: event.target.checked })}
            />
            <span>{t('linear.complex')}</span>
          </label>
        </div>

        {input.tab === 'equations' && (
          <SystemField
            lines={input.lines}
            onChange={(lines) => update({ ...latest.current, lines })}
            onSubmit={() => void solve()}
          />
        )}

        {error && <ErrorRow error={error} soft />}
        {!solution && !error && <p className="analysis-empty">{t(input.tab === 'equations' ? 'linear.empty.equations' : 'linear.empty.matrix')}</p>}

        {solution && (
          <section className="section-card linear-result">
            <div className="linear-result-head">
              <Katex className="section-result" latex={solution.result} />
              <button className="copy-btn" title={t('linear.copy.hint')} onClick={() => void copyAll()}>
                {copied ? t('copied') : t('copy')}
              </button>
            </div>
            <Steps steps={solution.steps} hidden={stepDetail === 'none'} />
          </section>
        )}
      </div>
    </main>
  );
}
