import { useCallback, useEffect, useRef, useState } from 'react';
import type { CSSProperties } from 'react';
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
  type SplineEnds,
  type SplineResult,
} from '../api';
import { useT } from '../i18n';
import { asDefinition, assignNames } from '../functionNames';
import { isPointList } from '../pointList';
import { copyText } from '../shell';
import type { TextKey } from '../i18n';
import { FunctionPlot } from './FunctionPlot';
import { displayStyle, Katex, latexLines } from './Katex';
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

/** `display`: the formulas in display style, so fractions stand at full size. */
export function Steps({
  steps,
  hidden,
  display = false,
  active,
  onHover,
}: {
  steps: Step[];
  hidden: boolean;
  display?: boolean;
  /** The step under the pointer, marked. */
  active?: number | null;
  onHover?(index: number | null): void;
}) {
  if (hidden) return null;
  return (
    <div className="analysis-steps">
      {steps.map((step, index) => (
        <div
          className={index === active ? 'analysis-step active' : 'analysis-step'}
          key={index}
          onMouseEnter={onHover && (() => onHover(index))}
          onMouseLeave={onHover && (() => onHover(null))}
        >
          {step.explanation && <span className="step-explanation">{step.explanation}</span>}
          {step.latex && <Katex className="step-latex" latex={display ? displayStyle(step.latex) : step.latex} />}
        </div>
      ))}
    </div>
  );
}

/** `soft`: grey instead of red, for input that is still being typed. */
export function ErrorRow({ error, soft = false }: { error: ApiError; soft?: boolean }) {
  const { t } = useT();
  return (
    <div className={soft ? 'error-row soft' : 'error-row'}>
      <span className="error-label">{t('error')}</span>
      <span className="error-text">
        {error.message}
        {error.position !== null && <span className="error-position"> · {t('position', { n: error.position + 1 })}</span>}
      </span>
    </div>
  );
}

export const asError = (failure: unknown) => (failure instanceof ApiError ? failure : new ApiError(String(failure)));

/** Identifies a point across re-runs, unlike its index. */
const pointKey = (point: MarkedPoint) => `${point.kind}|${point.x}|${point.y}`;

/**
 * The analysis mode: a function in, the curve sketch with its working on the left and the graph
 * on the right, plus tangent, area and value table as tasks on the same function.
 */
export function AnalysisView({ settings }: { settings: Settings }) {
  const { t } = useT();
  const ctrl = t('key.ctrl');
  const [copied, setCopied] = useState(false);
  const copyInput = async () => {
    await copyText(input);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1200);
  };
  const [input, setInput] = useState('');
  /** The domain field: empty for the function's own domain. */
  const [domainInput, setDomainInput] = useState('');
  /** A list of points in the input: the curve through them, how it ends, and its working. */
  const [ends, setEnds] = useState<SplineEnds>('natural');
  const [slopes, setSlopes] = useState({ start: '0', end: '0' });
  const [spline, setSpline] = useState<SplineResult | null>(null);
  const pointInput = isPointList(input);
  /** The other functions: drawn too, and usable in each other (`f(x - 1)`, `g'(x)`). */
  const [others, setOthers] = useState<string[]>([]);
  const names = assignNames([input, ...others]);
  /** The other functions as definitions for the one in the main field. */
  const definitionsFor = (all: string[], skip: number) =>
    all.flatMap((latex, i) => (i === skip || !latex.trim() || isPointList(latex) ? [] : [asDefinition(latex, assignNames(all)[i])]));
  /** The definitions the shown results belong to. */
  const [analysedDefinitions, setAnalysedDefinitions] = useState<string[]>([]);
  const [latexMode, setLatexMode] = useState(false);
  /** The function the results belong to; tasks and the graph use it, not the half-typed input. */
  const [analysed, setAnalysed] = useState<string | null>(null);
  /** The domain the results belong to. */
  const [analysedDomain, setAnalysedDomain] = useState<string | null>(null);
  const [analysis, setAnalysis] = useState<Analysis | null>(null);
  const [error, setError] = useState<ApiError | null>(null);
  const [tab, setTab] = useState<Tab>('sketch');
  /** The sections whose steps are shown; all closed at first, so the results read as a summary. */
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [selected, setSelected] = useState<string | null>(null);

  const [x0, setX0] = useState('1');
  const [tangent, setTangent] = useState<TangentResult | null>(null);
  const [bounds, setBounds] = useState({ from: '', to: '' });
  const [area, setArea] = useState<AreaResult | null>(null);
  const [table, setTable] = useState({ from: '-3', to: '3', step: '1' });
  const [rows, setRows] = useState<TableResult | null>(null);
  const [taskError, setTaskError] = useState<ApiError | null>(null);

  const field = useRef<MathFieldHandle>(null);
  const latexField = useRef<LatexInputHandle>(null);
  /** The latest request of each kind; an answer to an older one is dropped. */
  const analysisRequest = useRef(0);
  const taskRequest = useRef(0);

  const clearTasks = useCallback(() => {
    taskRequest.current++;
    setTangent(null);
    setArea(null);
    setRows(null);
    setTaskError(null);
  }, []);

  const run = useCallback(
    async (latex: string, domain: string | null, definitions: string[]) => {
      const id = ++analysisRequest.current;
      try {
        const result = await api.analyze(latex, domain, definitions);
        if (id !== analysisRequest.current) return;
        setAnalysis(result);
        setAnalysed(latex);
        setAnalysedDomain(domain);
        setAnalysedDefinitions(definitions);
        setError(null);
      } catch (failure) {
        if (id !== analysisRequest.current) return;
        setAnalysis(null);
        setAnalysed(null);
        setSelected(null);
        setError(asError(failure));
        clearTasks();
      }
    },
    [clearTasks],
  );

  const runTask = useCallback(
    async (which: Tab, latex: string) => {
      const id = ++taskRequest.current;
      const current = () => id === taskRequest.current;
      try {
        setTaskError(null);
        if (which === 'tangent') {
          const result = await api.tangent(latex, x0, analysedDomain, analysedDefinitions);
          if (current()) setTangent(result);
        }
        if (which === 'area') {
          const result = await api.areaUnderCurve(latex, bounds.from.trim() || null, bounds.to.trim() || null, analysedDomain, analysedDefinitions);
          if (current()) setArea(result);
        }
        if (which === 'table') {
          const result = await api.valueTable(latex, table.from, table.to, table.step, analysedDomain, analysedDefinitions);
          if (current()) setRows(result);
        }
      } catch (failure) {
        if (current()) setTaskError(asError(failure));
      }
    },
    [x0, bounds, table, analysedDomain, analysedDefinitions],
  );

  /** The curve through the points, then its sketch like any function's. */
  const runSpline = useCallback(
    async (points: string) => {
      const id = ++analysisRequest.current;
      try {
        const curve = await api.spline(points, ends, ends === 'clamped' ? slopes.start : null, ends === 'clamped' ? slopes.end : null);
        if (id !== analysisRequest.current) return;
        setSpline(curve);
        await run(curve.function, null, []);
      } catch (failure) {
        if (id !== analysisRequest.current) return;
        setSpline(null);
        setAnalysis(null);
        setAnalysed(null);
        setError(asError(failure));
        clearTasks();
      }
    },
    [ends, slopes, run, clearTasks],
  );

  const submit = () => {
    const latex = input.trim();
    if (!latex) return;
    clearTasks();
    setSelected(null);
    if (isPointList(latex)) {
      setExpanded(new Set(['spline']));
      void runSpline(latex);
      return;
    }
    setSpline(null);
    const domain = domainInput.trim() || null;
    if (latex !== analysed || domain !== analysedDomain) setExpanded(new Set());
    void run(latex, domain, definitionsFor([latex, ...others], 0));
  };

  /** Another function into the main field, the one there into its place, and sketched. */
  const sketchOther = (index: number) => {
    const chosen = others[index];
    const rest = [...others];
    rest[index] = input;
    setOthers(rest);
    setInput(chosen);
    const latex = chosen.trim();
    if (!latex) return;
    clearTasks();
    setSelected(null);
    setExpanded(new Set());
    setSpline(null);
    if (isPointList(latex)) {
      void runSpline(latex);
      return;
    }
    const domain = domainInput.trim() || null;
    void run(latex, domain, definitionsFor([latex, ...rest], 0));
  };

  const changeTab = (next: Tab) => {
    setTab(next);
    setSelected(null);
    setTaskError(null);
  };

  // A changed language, precision, output style or step detail shows at once.
  const { language, precision, binomialForm, denominatorForm, quadraticFormula, stepDetail, cancelMarks } = settings;
  const hideSteps = stepDetail === 'none';
  const latest = useRef({ analysed, analysedDomain, analysedDefinitions, tab, runTask, spline, input, runSpline });
  latest.current = { analysed, analysedDomain, analysedDefinitions, tab, runTask, spline, input, runSpline };
  useEffect(() => {
    const { analysed: latex, analysedDomain: domain, analysedDefinitions: definitions, tab: active, runTask: task, spline: curve, input: typed, runSpline: again } =
      latest.current;
    if (!latex) return;
    if (curve) void again(typed.trim());
    else void run(latex, domain, definitions);
    if (active !== 'sketch') void task(active, latex);
  }, [language, precision, binomialForm, denominatorForm, quadraticFormula, stepDetail, cancelMarks, run]);

  const toggleLatexMode = (seed?: string) => {
    setLatexMode((current) => {
      if (!current) setInput((text) => seed ?? toReadableLatex(text));
      return !current;
    });
  };

  const toggleSection = (id: string) =>
    setExpanded((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });

  const taskPoints: MarkedPoint[] = tab === 'tangent' && tangent ? [tangent.point] : [];
  const points = analysis ? [...analysis.points, ...taskPoints] : [];
  const selectedIndex = selected === null ? null : points.findIndex((point) => pointKey(point) === selected);
  const select = (index: number | null) => setSelected(index === null || index < 0 ? null : pointKey(points[index]));

  return (
    <main className="workspace analysis-workspace">
      <div className="analysis-panel">
        <div className={latexMode ? 'input-row latex' : 'input-row'} title={t('analysis.input.function')}>
          {latexMode ? (
            <LatexInput ref={latexField} value={input} onChange={setInput} onEnter={submit} onLeave={() => toggleLatexMode()} placeholder={PLACEHOLDER} />
          ) : (
            <MathField
              ref={field}
              value={input}
              onChange={setInput}
              onEnter={submit}
              onLatexRequested={(seed) => toggleLatexMode(seed)}
              placeholder={PLACEHOLDER}
            />
          )}
          <button className="mode-toggle" title={latexMode ? t('editor.back') : t('editor.latex', { ctrl })} onClick={() => toggleLatexMode()}>
            {latexMode ? 'TeX' : 'f(x)'}
          </button>
        </div>
        {pointInput ? (
          <div className="domain-row spline-row">
            <div className="segmented">
              {(['natural', 'clamped', 'polynomial'] as const).map((kind) => (
                <button key={kind} className={kind === ends ? 'segment active' : 'segment'} onClick={() => setEnds(kind)}>
                  {t(`analysis.spline.${kind}`)}
                </button>
              ))}
            </div>
            {ends === 'clamped' && (
              <>
                <label>
                  {t('analysis.spline.startSlope')}
                  <input className="task-input" value={slopes.start} onChange={(e) => setSlopes({ ...slopes, start: e.target.value })} onKeyDown={(e) => e.key === 'Enter' && submit()} />
                </label>
                <label>
                  {t('analysis.spline.endSlope')}
                  <input className="task-input" value={slopes.end} onChange={(e) => setSlopes({ ...slopes, end: e.target.value })} onKeyDown={(e) => e.key === 'Enter' && submit()} />
                </label>
              </>
            )}
          </div>
        ) : (
        <label className="domain-row" title={t('analysis.domain.hint')}>
          <span className="domain-row-label">
            <Katex latex="D =" />
          </span>
          <input
            className="task-input domain-input"
            value={domainInput}
            placeholder={t('analysis.domain.placeholder')}
            onChange={(e) => setDomainInput(e.target.value)}
            onKeyDown={(e) => e.key === 'Enter' && submit()}
            spellCheck={false}
          />
        </label>
        )}
        <div className="other-functions">
          {others.map((latex, i) => (
            <div className="other-function" key={i}>
              <span className="other-function-name">
                <Katex latex={`${names[i + 1]}(x) =`} />
              </span>
              <MathField
                className="other-function-field"
                value={latex}
                onChange={(value) => setOthers((all) => all.map((old, j) => (j === i ? value : old)))}
                onEnter={submit}
                placeholder="x^2"
              />
              <button className="copy-btn" title={t('analysis.functions.sketchHint')} onClick={() => sketchOther(i)}>
                {t('analysis.functions.sketch')}
              </button>
              <button className="copy-btn" title={t('analysis.functions.remove')} aria-label={t('analysis.functions.remove')} onClick={() => setOthers((all) => all.filter((_, j) => j !== i))}>
                ✕
              </button>
            </div>
          ))}
          <button className="copy-btn add-function" onClick={() => setOthers((all) => [...all, ''])} title={t('analysis.functions.addHint')}>
            + {t('analysis.functions.add')}
          </button>
        </div>
        {/* The other view of the input below it, as in the calculator: typeset while writing LaTeX, the source otherwise. */}
        <div className="source-row" style={{ '--lines': latexMode && input ? latexLines(input) : 1 } as CSSProperties}>
          {latexMode && input ? <Katex className="source-preview" latex={`\\displaystyle ${input}`} /> : <code className="source">{input || PLACEHOLDER}</code>}
          {input && (
            <button className="copy-btn" onClick={() => void copyInput()}>
              {copied ? t('copied') : t('analysis.input.copy')}
            </button>
          )}
        </div>

        <div className="segmented analysis-tabs">
          {TABS.map((item) => (
            <button key={item.id} className={item.id === tab ? 'segment active' : 'segment'} onClick={() => changeTab(item.id)}>
              {t(item.label)}
            </button>
          ))}
        </div>

        {error && <ErrorRow error={error} />}
        {!analysis && !error && <p className="analysis-empty">{t('analysis.empty')}</p>}

        {analysis && analysed && (
          <div className="analysis-body">
            <div className="analysis-graph">
              <FunctionPlot
                latex={analysed}
                domain={analysedDomain}
                definitions={analysedDefinitions}
                plot={analysis.plot}
                points={points}
                asymptotes={analysis.asymptotes}
                lines={tab === 'tangent' && tangent ? [tangent.tangent, tangent.normal] : []}
                regions={tab === 'area' && area ? area.regions : []}
                dots={[...(spline ? spline.points : []), ...(tab === 'table' && rows ? tableDots(rows) : [])]}
                selected={selectedIndex !== null && selectedIndex >= 0 ? selectedIndex : null}
                onSelect={select}
              />
            </div>
            <div className="analysis-sheet">
              <div className="analysis-function">
                <Katex latex={analysis.function} display />
                {analysis.period && (
                  <span className="analysis-period">
                    {t('analysis.period')} <Katex latex={analysis.period} />
                  </span>
                )}
              </div>

              {tab === 'sketch' && spline && (
                <section className="section-card">
                  <button className="section-head" disabled={hideSteps} aria-expanded={!hideSteps ? expanded.has('spline') : undefined} onClick={() => toggleSection('spline')}>
                    {!hideSteps && <span className={expanded.has('spline') ? 'chev open' : 'chev'}>⌄</span>}
                    <span className="section-title">{t(`analysis.spline.${ends}`)}</span>
                  </button>
                  <Katex className="section-result" latex={`f(x) = ${spline.function}`} />
                  {expanded.has('spline') && <Steps steps={spline.steps} hidden={hideSteps} />}
                </section>
              )}
              {tab === 'sketch' &&
                analysis.sections.map((section) => {
                  const open = expanded.has(section.id);
                  const hasSteps = section.steps.length > 0 && !hideSteps;
                  return (
                    <section className="section-card" key={section.id}>
                      <button className="section-head" disabled={!hasSteps} aria-expanded={hasSteps ? open : undefined} onClick={() => toggleSection(section.id)}>
                        {hasSteps && <span className={open ? 'chev open' : 'chev'}>⌄</span>}
                        <span className="section-title">{section.title}</span>
                        {section.status !== 'ok' && (
                          <span className={`section-status ${section.status}`} title={section.message ?? ''}>
                            {t(section.status === 'partial' ? 'analysis.status.partial' : 'analysis.status.failed')}
                          </span>
                        )}
                      </button>
                      {section.result && <Katex className="section-result" latex={section.result} />}
                      {section.message && <p className="section-message">{section.message}</p>}
                      {open && hasSteps && <Steps steps={section.steps} hidden={hideSteps} />}
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
                      <Steps steps={tangent.steps} hidden={hideSteps} />
                    </>
                  )}
                </section>
              )}

              {tab === 'area' && (
                <section className="section-card">
                  <div className="task-inputs">
                    <label>
                      {t('analysis.from')}
                      <input
                        className="task-input"
                        value={bounds.from}
                        onChange={(e) => setBounds({ ...bounds, from: e.target.value })}
                        onKeyDown={(e) => e.key === 'Enter' && void runTask('area', analysed)}
                      />
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
                      <Steps steps={area.steps} hidden={hideSteps} />
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

          </div>
        )}
      </div>
    </main>
  );
}

/** The table rows inside the domain as dots for the graph. */
function tableDots(rows: TableResult): [number, number][] {
  return rows.rows.flatMap((row) => (row.yValue === null ? [] : [[row.xValue, row.yValue] as [number, number]]));
}
