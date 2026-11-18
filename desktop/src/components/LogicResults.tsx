import { useState, type ReactNode } from 'react';
import type { DrawForm, GateFormResult, LogicForm, LogicNotation, LogicOutputResult, LogicScene, LogicSymbols, Worked } from '../api';
import { nameLatex, type Shown, type TermHover } from '../logic';
import { useT, type TextKey } from '../i18n';
import { copyText } from '../shell';
import { Steps } from './AnalysisView';
import { displayStyle, Katex } from './Katex';
import { SourceMark } from './LogicLines';
import { LogicGates } from './LogicGates';
import { editorData, withGateChoice } from '../logicCircuit';

interface Props {
  scene: LogicScene;
  /** The output whose results show. */
  shown: Shown;
  /** At the detail level `none` the steps are hidden. */
  hideSteps: boolean;
  /** How the terms of a minimal form are joined. */
  notation: LogicNotation;
  /** The gate symbols of the Gates card's picture. */
  symbols: LogicSymbols;
  /** **Open in editor** of the Gates card. */
  onOpen(form: DrawForm, output: string, fromExpression: boolean): void;
  /** The term hovered here or in the KV diagram. */
  hover: TermHover | null;
  onHover(hover: TermHover | null): void;
  onChange(scene: LogicScene): void;
}

/**
 * A minimal form `f = …` term by term, so that hovering a term singles out its block in the KV
 * diagram, and the other way round. Without terms (a constant) the form as the core wrote it.
 */
function Terms({ output, form, which, notation, hover, onHover }: { output: string; form: LogicForm; which: 'dnf' | 'knf'; notation: LogicNotation; hover: TermHover | null; onHover(hover: TermHover | null): void }) {
  const name = nameLatex(output);
  if (!form.terms.length) return <Katex className="section-result" latex={displayStyle(`${name} = ${form.latex}`)} />;
  const technical = notation === 'technical';
  const join = which === 'dnf' ? (technical ? '+' : '\\lor') : technical ? '\\cdot' : '\\land';
  const literals = (pattern: string) => [...pattern].filter((c) => c !== '-').length;
  const active = hover && hover.output === output && hover.form === which ? hover.term : null;
  return (
    <span className="section-result logic-terms">
      <Katex latex={displayStyle(`${name} =`)} />
      {form.terms.map((term, k) => (
        <span key={k} className="logic-terms-part">
          {k > 0 && <Katex latex={displayStyle(join)} />}
          <span
            className={active === k ? 'logic-term active' : 'logic-term'}
            style={{ ['--block' as string]: `var(--geometry-${k % 8})` }}
            onMouseEnter={() => onHover({ output, form: which, term: k })}
            onMouseLeave={() => onHover(null)}
          >
            <Katex latex={displayStyle(which === 'knf' && form.terms.length > 1 && literals(term.pattern) > 1 ? `(${term.latex})` : term.latex)} />
          </span>
        </span>
      ))}
    </span>
  );
}

/** Copy for a LaTeX block, saying *Copied* for a moment. */
export function CopyButton({ text, hint }: { text: string; hint?: string }) {
  const { t } = useT();
  const [copied, setCopied] = useState(false);
  if (!text) return null;
  return (
    <button
      className="copy-btn"
      title={hint}
      onClick={() =>
        void copyText(text).then(() => {
          setCopied(true);
          window.setTimeout(() => setCopied(false), 1200);
        })
      }
    >
      {copied ? t('copied') : t('copy')}
    </button>
  );
}

/** A result: its title, its formulas, a line of figures, the choice among several, Copy and the steps to open. */
export function Section({
  title,
  formula,
  lines,
  meta,
  choice,
  worked,
  more,
  hideSteps,
  bare,
  children,
}: {
  title: TextKey;
  /** Shown before the lines. */
  formula?: ReactNode;
  lines: string[];
  meta?: string;
  choice?: ReactNode;
  worked: Worked | null;
  /** A second working, opened on its own (Quine–McCluskey beside the KV path). */
  more?: { title: TextKey; worked: Worked };
  hideSteps: boolean;
  /** Shown under the result, above the steps. */
  children?: ReactNode;
  /** Without a card of its own: one of two columns in a shared card. */
  bare?: boolean;
}) {
  const { t } = useT();
  const [open, setOpen] = useState<'steps' | 'more' | null>(null);
  const toggle = (which: 'steps' | 'more') => setOpen((current) => (current === which ? null : which));
  const shown = open === 'more' ? more?.worked : worked;
  return (
    <section className={bare ? 'logic-result' : 'section-card logic-result'}>
      <div className="linear-result-head">
        <div className="logic-result-body">
          <span className="section-title">{t(title)}</span>
          {formula}
          {lines.map((line, i) => (
            <Katex key={i} className="section-result" latex={displayStyle(line)} />
          ))}
          {meta && <span className="logic-result-meta">{meta}</span>}
          {choice}
        </div>
        <CopyButton text={shown?.alignBlock ?? worked?.alignBlock ?? ''} hint={t('logic.copy.hint')} />
      </div>
      {children}
      {!hideSteps && (worked?.steps.length || more) ? (
        <div className="form-row logic-result-toggles">
          {worked && worked.steps.length > 0 && (
            <button className={open === 'steps' ? 'form-chip active' : 'form-chip'} onClick={() => toggle('steps')}>
              {t('steps')}
            </button>
          )}
          {more && (
            <button className={open === 'more' ? 'form-chip active' : 'form-chip'} onClick={() => toggle('more')}>
              {t(more.title)}
            </button>
          )}
        </div>
      ) : null}
      {shown && open && <Steps steps={shown.steps} hidden={hideSteps} display />}
    </section>
  );
}

/**
 * The results of one output at a time (§10.1, panel 4), in three cards: the expression it is
 * given by beside the canonical forms, the minimal DNF beside the minimal KNF with the choice
 * among several, and the Gates card with the six gate forms, each with its steps and Copy.
 */
export function LogicResults({ scene, shown, hideSteps, notation, symbols, onOpen, hover, onHover, onChange }: Props) {
  const { t } = useT();
  const { result, index } = shown;
  const name = nameLatex(result.name);
  const eq = (latex: string) => `${name} = ${latex}`;

  /** Picks minimal solution `k`; a lamp of the circuit gets an output of its own to keep the choice. */
  const choose = (key: 'chosen' | 'chosenKnf', k: number) => {
    if (index !== null) onChange({ ...scene, outputs: scene.outputs.map((o, i) => (i === index ? { ...o, [key]: k } : o)) });
    else onChange({ ...scene, outputs: [...scene.outputs, { name: result.name, source: { kind: 'circuit' }, [key]: k }] });
  };

  const cost = (form: LogicForm) => t('logic.cost', { terms: form.cost.terms, literals: form.cost.literals });
  const gates = (form: GateFormResult) => t('logic.gates', { gates: form.gates, inputs: form.inputs, levels: form.levels });
  const choices = (forms: LogicForm[], chosen: number, key: 'chosen' | 'chosenKnf') =>
    forms.length > 1 ? (
      <div className="form-row logic-choices" title={t('logic.minimal.choice.hint')}>
        <span className="logic-result-meta">{t('logic.minimal.choice', { count: forms.length })}</span>
        {forms.map((_, k) => (
          <button key={k} className={k === chosen ? 'form-chip active' : 'form-chip'} onClick={() => choose(key, k)}>
            {k + 1}
          </button>
        ))}
      </div>
    ) : undefined;

  return (
    <div className="logic-results">
      <ResultsOf
        result={result}
        eq={eq}
        cost={cost}
        gates={gates}
        choices={choices}
        gatesCard={(props) => (
          <LogicGates
            scene={scene}
            result={result}
            symbols={symbols}
            choice={editorData(scene).gates?.[result.name]}
            onChoose={(choice) => onChange(withGateChoice(scene, result.name, choice))}
            onOpen={(form, fromExpression) => onOpen(form, result.name, fromExpression)}
            {...props}
          />
        )}
        hideSteps={hideSteps}
        terms={(form, which) => <Terms output={result.name} form={form} which={which} notation={notation} hover={hover} onHover={onHover} />}
      />
    </div>
  );
}

function ResultsOf({
  result,
  eq,
  cost,
  gates,
  choices,
  gatesCard,
  hideSteps,
  terms,
}: {
  result: LogicOutputResult;
  eq(latex: string): string;
  cost(form: LogicForm): string;
  gates(form: GateFormResult): string;
  choices(forms: LogicForm[], chosen: number, key: 'chosen' | 'chosenKnf'): ReactNode;
  /** The Gates card, given what ResultsOf shares with the other cards. */
  gatesCard(props: { eq(latex: string): string; cost(form: LogicForm): string; gates(form: GateFormResult): string; hideSteps: boolean }): ReactNode;
  hideSteps: boolean;
  terms(form: LogicForm, which: 'dnf' | 'knf'): ReactNode;
}) {
  const { t } = useT();
  const { expression, canonical, minimal } = result;
  const dnf = minimal?.dnf[minimal.chosen];
  const knf = minimal?.knf[minimal.chosenKnf];
  return (
    <>
      <div className="logic-results-head">
        <SourceMark kind={result.source} />
        {result.constant && <span className="section-message">{result.constant}</span>}
      </div>
      {(expression || canonical) && (
        <div className="section-card logic-card-pair">
          {expression && (
            <Section
              bare
              title={result.source === 'circuit' ? 'logic.result.traced' : 'logic.result.expression'}
              lines={[eq(expression.latex)]}
              worked={expression.worked}
              more={result.tableWorked ? { title: 'logic.result.table', worked: result.tableWorked } : undefined}
              hideSteps={hideSteps}
            />
          )}
          {canonical && (
            <Section
              bare
              title="logic.result.canonical"
              lines={[eq(canonical.sum), eq(canonical.dnf), eq(canonical.product), eq(canonical.knf)]}
              worked={canonical.worked}
              hideSteps={hideSteps}
            />
          )}
        </div>
      )}
      {minimal && (dnf || knf) && (
        <div className="section-card logic-card-pair">
          {dnf && (
            <Section
              bare
              title="logic.result.dnf"
              formula={terms(dnf, 'dnf')}
              lines={[]}
              meta={cost(dnf)}
              choice={choices(minimal.dnf, minimal.chosen, 'chosen')}
              worked={minimal.worked}
              more={{ title: 'logic.result.quineMcCluskey', worked: minimal.quineMcCluskey }}
              hideSteps={hideSteps}
            />
          )}
          {knf && (
            <Section
              bare
              title="logic.result.knf"
              formula={terms(knf, 'knf')}
              lines={[]}
              meta={cost(knf)}
              choice={choices(minimal.knf, minimal.chosenKnf, 'chosenKnf')}
              worked={minimal.knfWorked}
              hideSteps={hideSteps}
            />
          )}
        </div>
      )}
      {gatesCard({ eq, cost, gates, hideSteps })}
      {!expression && !canonical && <p className="analysis-empty">{t('logic.result.none')}</p>}
    </>
  );
}
