import { useCallback, useEffect, useRef, useState } from 'react';
import { api, ApiError, type LinearSystem, type Settings } from '../api';
import { matrixLatex, requestFor, withMatrix, type LinearInput, type LinearTab } from '../linear';
import { useT } from '../i18n';
import { copyText } from '../shell';
import type { TextKey } from '../i18n/en';
import { asError, ErrorRow, Steps } from './AnalysisView';
import { Katex } from './Katex';
import { MatrixGrid } from './MatrixGrid';
import { MatrixJson } from './MatrixJson';
import { SystemField } from './SystemField';

const TABS: { id: LinearTab; label: TextKey }[] = [
  { id: 'equations', label: 'linear.tab.equations' },
  { id: 'matrix', label: 'linear.tab.matrix' },
];

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
  /** The request the solution answers, to tell whether the input has changed since. */
  const solved = useRef<string | null>(null);
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
      solved.current = JSON.stringify(next);
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

  /**
   * The other tab gets the system as the engine read it: the matrix and its unknowns, or one
   * equation per row. That needs a solution of the input as it stands; without one, the other tab
   * keeps its own input.
   */
  const changeTab = (tab: LinearTab) => {
    const current = latest.current;
    if (tab === current.tab) return;
    const converts = solution !== null && solved.current === JSON.stringify(requestFor(current));
    let next: LinearInput = { ...current, tab };
    if (converts && tab === 'matrix') next = withMatrix(next, { unknowns: solution.unknowns, rows: solution.matrix });
    if (converts && tab === 'equations') next = { ...next, lines: solution.equations };
    update(next);
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

        {input.tab === 'matrix' &&
          (input.large ? (
            <section className="section-card matrix-large">
              <div className="task-inputs">
                <span>{t('linear.large', { m: input.large.rows.length, n: input.large.rows[0].length - 1 })}</span>
                <button className="palette-item" onClick={() => update({ ...latest.current, large: null })}>
                  {t('linear.large.back')}
                </button>
              </div>
              <Katex className="matrix-preview" latex={matrixLatex(input.large)} display />
            </section>
          ) : (
            <MatrixGrid
              grid={input.grid}
              names={unknownNames}
              onChange={(grid) => update({ ...latest.current, grid })}
              onSubmit={() => void solve()}
            />
          ))}
        {input.tab === 'matrix' && (
          <MatrixJson
            onLoad={(matrix) => {
              update(withMatrix(latest.current, matrix));
              void solve();
            }}
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
