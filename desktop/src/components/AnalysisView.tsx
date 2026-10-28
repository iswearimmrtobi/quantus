import { useCallback, useEffect, useRef, useState } from 'react';
import {
  api,
  ApiError,
  type Analysis,
  type AreaResult,
  type MarkedPoint,
  type Settings,
  type Step,
  type TableResult,
  type TangentResult,
} from '../api';
import { useT } from '../i18n';
import type { TextKey } from '../i18n/en';
import { FunctionPlot } from './FunctionPlot';
import { Katex } from './Katex';
import { LatexInput, type LatexInputHandle } from './LatexInput';
import { MathField, toReadableLatex, type MathFieldHandle } from './MathField';

type Tab = 'sketch' | 'tangent' | 'area' | 'table';

const TABS: { id: Tab; label: TextKey }[] = [
  { id: 'sketch', label: 'analysis.tab.sketch' },
  { id: 'tangent', label: 'analysis.tab.tangent' },
  { id: 'area', label: 'analysis.tab.area' },
  { id: 'table', label: 'analysis.tab.table' },
];

const PLACEHOLDER = 'f(x) = x^3 - 3x';

function Steps({ steps }: { steps: Step[] }) {
  return (
    <div className="analysis-steps">
      {steps.map((step, index) => (
        <div className="analysis-step" key={index}>
          {step.explanation && <span className="step-explanation">{step.explanation}</span>}
          {step.latex && <Katex className="step-latex" latex={step.latex} />}
        </div>
      ))}
    </div>
  );
}

function ErrorRow({ error }: { error: ApiError }) {
  const { t } = useT();
  return (
    <div className="error-row">
      <span className="error-label">{t('error')}</span>
      <span className="error-text">
        {error.message}
        {error.position !== null && <span className="error-position"> · {t('position', { n: error.position + 1 })}</span>}
      </span>
    </div>
  );
}

const asError = (failure: unknown) => (failure instanceof ApiError ? failure : new ApiError(String(failure)));

/**
 * The analysis mode: a function in, the curve sketch with its working on the left and the graph
 * on the right, plus tangent, area and value table as tasks on the same function.
 */
export function AnalysisView({ settings }: { settings: Settings }) {
  const { t } = useT();
  const [input, setInput] = useState('');
  const [latexMode, setLatexMode] = useState(false);
  /** The function the results belong to; tasks and the graph use it, not the half-typed input. */
  const [analysed, setAnalysed] = useState<string | null>(null);
  const [analysis, setAnalysis] = useState<Analysis | null>(null);
  const [error, setError] = useState<ApiError | null>(null);
  const [tab, setTab] = useState<Tab>('sketch');
  const [collapsed, setCollapsed] = useState<Set<string>>(new Set());
  const [selected, setSelected] = useState<number | null>(null);

  const [x0, setX0] = useState('1');
  const [tangent, setTangent] = useState<TangentResult | null>(null);
  const [bounds, setBounds] = useState({ from: '', to: '' });
  const [area, setArea] = useState<AreaResult | null>(null);
  const [table, setTable] = useState({ from: '-3', to: '3', step: '1' });
  const [rows, setRows] = useState<TableResult | null>(null);
  const [taskError, setTaskError] = useState<ApiError | null>(null);

  const field = useRef<MathFieldHandle>(null);
  const latexField = useRef<LatexInputHandle>(null);

  const run = useCallback(async (latex: string) => {
    try {
      setAnalysis(await api.analyze(latex));
      setAnalysed(latex);
      setError(null);
      setSelected(null);
    } catch (failure) {
      setAnalysis(null);
      setAnalysed(null);
      setError(asError(failure));
    }
  }, []);

  const runTask = useCallback(
    async (which: Tab, latex: string) => {
      try {
        setTaskError(null);
        if (which === 'tangent') setTangent(await api.tangent(latex, x0));
        if (which === 'area') setArea(await api.areaUnderCurve(latex, bounds.from.trim() || null, bounds.to.trim() || null));
        if (which === 'table') setRows(await api.valueTable(latex, table.from, table.to, table.step));
      } catch (failure) {
        setTaskError(asError(failure));
      }
    },
    [x0, bounds, table],
  );

  const submit = () => {
    const latex = input.trim();
    if (!latex) return;
    setTangent(null);
    setArea(null);
    setRows(null);
    setTaskError(null);
    void run(latex);
  };

  // A changed language, precision or output style shows at once.
  const { language, precision, binomialForm, denominatorForm, quadraticFormula } = settings;
  const latest = useRef({ analysed, tab, runTask });
  latest.current = { analysed, tab, runTask };
  useEffect(() => {
    const { analysed: latex, tab: active, runTask: task } = latest.current;
    if (!latex) return;
    void run(latex);
    if (active !== 'sketch') void task(active, latex);
  }, [language, precision, binomialForm, denominatorForm, quadraticFormula, run]);

  const toggleLatexMode = (seed?: string) => {
    setLatexMode((current) => {
      if (!current) setInput((text) => seed ?? toReadableLatex(text));
      return !current;
    });
  };

  const toggleSection = (id: string) =>
    setCollapsed((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });

  const taskPoints: MarkedPoint[] = tab === 'tangent' && tangent ? [tangent.point] : [];
  const points = analysis ? [...analysis.points, ...taskPoints] : [];

  return (
    <main className="workspace analysis-workspace">
      <div className="analysis-panel">
        <div className={latexMode ? 'input-row latex' : 'input-row'}>
          {latexMode ? (
            <LatexInput ref={latexField} value={input} onChange={setInput} onEnter={submit} onLeave={() => toggleLatexMode()} />
          ) : (
            <MathField ref={field} value={input} onChange={setInput} onEnter={submit} onLatexRequested={(seed) => toggleLatexMode(seed)} />
          )}
          <button className="mode-toggle" onClick={() => toggleLatexMode()}>
            {latexMode ? 'TeX' : 'f(x)'}
          </button>
        </div>
        <div className="source-row">
          <code className="source">{input || PLACEHOLDER}</code>
        </div>

        <div className="segmented analysis-tabs">
          {TABS.map((item) => (
            <button key={item.id} className={item.id === tab ? 'segment active' : 'segment'} onClick={() => setTab(item.id)}>
              {t(item.label)}
            </button>
          ))}
        </div>

        {error && <ErrorRow error={error} />}
        {!analysis && !error && <p className="analysis-empty">{t('analysis.empty')}</p>}

        {analysis && analysed && (
          <div className="analysis-body">
            <div className="analysis-sheet">
              <div className="analysis-function">
                <Katex latex={analysis.function} display />
                {analysis.period && (
                  <span className="analysis-period">
                    {t('analysis.period')} <Katex latex={analysis.period} />
                  </span>
                )}
              </div>

              {tab === 'sketch' &&
                analysis.sections.map((section) => {
                  const open = !collapsed.has(section.id);
                  return (
                    <section className="section-card" key={section.id}>
                      <button className="section-head" onClick={() => toggleSection(section.id)}>
                        <span className={open ? 'chev open' : 'chev'}>⌄</span>
                        <span className="section-title">{section.title}</span>
                        {section.status !== 'ok' && (
                          <span className={`section-status ${section.status}`} title={section.message ?? ''}>
                            {t(section.status === 'partial' ? 'analysis.partial' : 'analysis.failed')}
                          </span>
                        )}
                      </button>
                      {section.result && <Katex className="section-result" latex={section.result} />}
                      {section.message && open && <p className="section-message">{section.message}</p>}
                      {open && section.steps.length > 0 && <Steps steps={section.steps} />}
                    </section>
                  );
                })}

              {tab === 'tangent' && (
                <section className="section-card">
                  <div className="task-inputs">
                    <label>
                      {t('analysis.x0')}
                      <input className="task-input" value={x0} onChange={(e) => setX0(e.target.value)} onKeyDown={(e) => e.key === 'Enter' && void runTask('tangent', analysed)} />
                    </label>
                    <button className="palette-item" onClick={() => void runTask('tangent', analysed)}>
                      {t('analysis.compute')}
                    </button>
                  </div>
                  {taskError && <ErrorRow error={taskError} />}
                  {tangent && (
                    <>
                      <Katex className="section-result" latex={`${tangent.tangent.latex}, \\quad ${tangent.normal.latex}`} />
                      <Steps steps={tangent.steps} />
                    </>
                  )}
                </section>
              )}

              {tab === 'area' && (
                <section className="section-card">
                  <div className="task-inputs">
                    <label>
                      {t('analysis.from')}
                      <input className="task-input" value={bounds.from} onChange={(e) => setBounds({ ...bounds, from: e.target.value })} />
                    </label>
                    <label>
                      {t('analysis.to')}
                      <input className="task-input" value={bounds.to} onChange={(e) => setBounds({ ...bounds, to: e.target.value })} onKeyDown={(e) => e.key === 'Enter' && void runTask('area', analysed)} />
                    </label>
                    <button className="palette-item" onClick={() => void runTask('area', analysed)}>
                      {t('analysis.compute')}
                    </button>
                  </div>
                  <p className="setting-hint">{t('analysis.area.hint')}</p>
                  {taskError && <ErrorRow error={taskError} />}
                  {area && (
                    <>
                      <div className="task-results">
                        <span className="domain-label">{t('analysis.area')}</span>
                        <Katex latex={`A = ${area.area}`} />
                        <span className="domain-label">{t('analysis.signed')}</span>
                        <Katex latex={area.signed} />
                      </div>
                      <Steps steps={area.steps} />
                    </>
                  )}
                </section>
              )}

              {tab === 'table' && (
                <section className="section-card">
                  <div className="task-inputs">
                    {(['from', 'to', 'step'] as const).map((key) => (
                      <label key={key}>
                        {t(`analysis.${key}`)}
                        <input
                          className="task-input"
                          value={table[key]}
                          onChange={(e) => setTable({ ...table, [key]: e.target.value })}
                          onKeyDown={(e) => e.key === 'Enter' && void runTask('table', analysed)}
                        />
                      </label>
                    ))}
                    <button className="palette-item" onClick={() => void runTask('table', analysed)}>
                      {t('analysis.compute')}
                    </button>
                  </div>
                  {taskError && <ErrorRow error={taskError} />}
                  {rows && (
                    <table className="value-table">
                      <thead>
                        <tr>
                          <th>
                            <Katex latex={analysis.variable} />
                          </th>
                          <th>
                            <Katex latex={`f(${analysis.variable})`} />
                          </th>
                        </tr>
                      </thead>
                      <tbody>
                        {rows.rows.map((row, i) => (
                          <tr key={i}>
                            <td>
                              <Katex latex={row.x} />
                            </td>
                            <td>{row.y === null ? <span className="table-undefined">{t('analysis.undefined')}</span> : <Katex latex={row.y} />}</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  )}
                </section>
              )}
            </div>

            <div className="analysis-graph">
              <FunctionPlot
                latex={analysed}
                plot={analysis.plot}
                points={points}
                asymptotes={analysis.asymptotes}
                lines={tab === 'tangent' && tangent ? [tangent.tangent, tangent.normal] : []}
                regions={tab === 'area' && area ? area.regions : []}
                dots={tab === 'table' && rows ? tableDots(rows) : []}
                selected={selected}
                onSelect={setSelected}
              />
              <div className="point-list">
                {points.map((point, i) => (
                  <button
                    key={i}
                    className={i === selected ? `point-item kind-${point.kind} selected` : `point-item kind-${point.kind}`}
                    onClick={() => setSelected(i === selected ? null : i)}
                  >
                    <span className="point-swatch" />
                    <span className="point-kind">{t(`point.${point.kind}`)}</span>
                    <Katex latex={point.latex} />
                  </button>
                ))}
              </div>
            </div>
          </div>
        )}
      </div>
    </main>
  );
}

/** The table rows as dots for the graph, where both coordinates read as numbers. */
function tableDots(rows: TableResult): [number, number][] {
  const number = (latex: string) => {
    const plain = latex.replace('{,}', '.').replace(/\\frac\{(-?\d+)\}\{(\d+)\}/, (_, a, b) => String(Number(a) / Number(b)));
    const value = Number(plain);
    return Number.isFinite(value) ? value : null;
  };
  return rows.rows.flatMap((row) => {
    const x = number(row.x);
    const y = row.y === null ? null : number(row.y);
    return x !== null && y !== null ? [[x, y] as [number, number]] : [];
  });
}
