import { useState, type ReactNode } from 'react';
import type { GateFormResult, LogicForm, LogicOutputResult, LogicScene, Worked } from '../api';
import { nameLatex, type LogicAnswer } from '../logic';
import { useT, type TextKey } from '../i18n';
import { copyText } from '../shell';
import { Steps } from './AnalysisView';
import { displayStyle, Katex } from './Katex';
import { SourceMark } from './LogicLines';

interface Props {
  scene: LogicScene;
  answer: LogicAnswer;
  /** At the detail level `none` the steps are hidden. */
  hideSteps: boolean;
  onChange(scene: LogicScene): void;
}

/** Copy for an `align*` block, saying *Copied* for a moment. */
function CopyButton({ text, hint }: { text: string; hint?: string }) {
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
function Section({
  title,
  lines,
  meta,
  choice,
  worked,
  more,
  hideSteps,
}: {
  title: TextKey;
  lines: string[];
  meta?: string;
  choice?: ReactNode;
  worked: Worked | null;
  /** A second working, opened on its own (Quine–McCluskey beside the KV path). */
  more?: { title: TextKey; worked: Worked };
  hideSteps: boolean;
}) {
  const { t } = useT();
  const [open, setOpen] = useState<'steps' | 'more' | null>(null);
  const toggle = (which: 'steps' | 'more') => setOpen((current) => (current === which ? null : which));
  const shown = open === 'more' ? more?.worked : worked;
  return (
    <section className="section-card logic-result">
      <div className="linear-result-head">
        <div className="logic-result-body">
          <span className="section-title">{t(title)}</span>
          {lines.map((line, i) => (
            <Katex key={i} className="section-result" latex={displayStyle(line)} />
          ))}
          {meta && <span className="logic-result-meta">{meta}</span>}
          {choice}
        </div>
        <CopyButton text={shown?.alignBlock ?? worked?.alignBlock ?? ''} hint={t('logic.copy.hint')} />
      </div>
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
 * The results of one output at a time (§10.1, panel 4): the expression it is given by, the
 * canonical forms, the minimal DNF and KNF with the choice among several, the NAND and NOR forms
 * with their cost, each with its steps and Copy.
 */
export function LogicResults({ scene, answer, hideSteps, onChange }: Props) {
  const { t } = useT();
  const results = [
    ...answer.outputs.flatMap((result, index) => (result ? [{ result, index: index as number | null }] : [])),
    ...answer.extra.map((result) => ({ result, index: null as number | null })),
  ];
  const [tab, setTab] = useState<string | null>(null);
  if (!results.length) return null;
  const { result, index } = results.find((r) => r.result.name === tab) ?? results[0];
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
      {results.length > 1 && (
        <div className="form-row logic-tabs">
          {results.map(({ result: r }) => (
            <button key={r.name} className={r.name === result.name ? 'form-chip active' : 'form-chip'} onClick={() => setTab(r.name)}>
              <Katex latex={nameLatex(r.name)} />
            </button>
          ))}
        </div>
      )}
      <ResultsOf result={result} eq={eq} cost={cost} gates={gates} choices={choices} hideSteps={hideSteps} />
    </div>
  );
}

function ResultsOf({
  result,
  eq,
  cost,
  gates,
  choices,
  hideSteps,
}: {
  result: LogicOutputResult;
  eq(latex: string): string;
  cost(form: LogicForm): string;
  gates(form: GateFormResult): string;
  choices(forms: LogicForm[], chosen: number, key: 'chosen' | 'chosenKnf'): ReactNode;
  hideSteps: boolean;
}) {
  const { t } = useT();
  const { expression, canonical, minimal, nand, nor, nandRewrite, norRewrite } = result;
  const dnf = minimal?.dnf[minimal.chosen];
  const knf = minimal?.knf[minimal.chosenKnf];
  return (
    <>
      <div className="logic-results-head">
        <SourceMark kind={result.source} />
        {result.constant && <span className="section-message">{result.constant}</span>}
      </div>
      {expression && (
        <Section
          title={result.source === 'circuit' ? 'logic.result.traced' : 'logic.result.expression'}
          lines={[eq(expression.latex)]}
          worked={expression.worked}
          more={result.tableWorked ? { title: 'logic.result.table', worked: result.tableWorked } : undefined}
          hideSteps={hideSteps}
        />
      )}
      {canonical && (
        <Section
          title="logic.result.canonical"
          lines={[eq(canonical.sum), eq(canonical.dnf), eq(canonical.product), eq(canonical.knf)]}
          worked={canonical.worked}
          hideSteps={hideSteps}
        />
      )}
      {minimal && dnf && (
        <Section
          title="logic.result.dnf"
          lines={[eq(dnf.latex)]}
          meta={cost(dnf)}
          choice={choices(minimal.dnf, minimal.chosen, 'chosen')}
          worked={minimal.worked}
          more={{ title: 'logic.result.quineMcCluskey', worked: minimal.quineMcCluskey }}
          hideSteps={hideSteps}
        />
      )}
      {minimal && knf && (
        <Section
          title="logic.result.knf"
          lines={[eq(knf.latex)]}
          meta={cost(knf)}
          choice={choices(minimal.knf, minimal.chosenKnf, 'chosenKnf')}
          worked={minimal.knfWorked}
          hideSteps={hideSteps}
        />
      )}
      {nand && <Section title="logic.result.nand" lines={[eq(nand.latex)]} meta={gates(nand)} worked={nand.worked} hideSteps={hideSteps} />}
      {nandRewrite && (
        <Section title="logic.result.nandRewrite" lines={[eq(nandRewrite.latex)]} meta={gates(nandRewrite)} worked={nandRewrite.worked} hideSteps={hideSteps} />
      )}
      {nor && <Section title="logic.result.nor" lines={[eq(nor.latex)]} meta={gates(nor)} worked={nor.worked} hideSteps={hideSteps} />}
      {norRewrite && (
        <Section title="logic.result.norRewrite" lines={[eq(norRewrite.latex)]} meta={gates(norRewrite)} worked={norRewrite.worked} hideSteps={hideSteps} />
      )}
      {!expression && !canonical && <p className="analysis-empty">{t('logic.result.none')}</p>}
    </>
  );
}
