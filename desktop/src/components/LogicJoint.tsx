import { useState } from 'react';
import type { JointForm, JointResult, LogicNotation, LogicScene } from '../api';
import { nameLatex } from '../logic';
import { useT } from '../i18n';
import { displayStyle, Katex } from './Katex';
import { Section } from './LogicResults';
import { Segmented } from './SettingsDialog';

/**
 * One output of a joint form, `f = …` term by term. A term keeps its colour in every output, a
 * shared one is marked, and hovering a term marks it in every output's line.
 */
function JointLine({ form, output, which, notation, hover, onHover }: { form: JointForm; output: number; which: 'dnf' | 'knf'; notation: LogicNotation; hover: number | null; onHover(term: number | null): void }) {
  const { t } = useT();
  const line = form.outputs[output];
  const name = nameLatex(line.name);
  if (!line.terms.length) return <Katex className="section-result" latex={displayStyle(`${name} = ${line.latex}`)} />;
  const technical = notation === 'technical';
  const join = which === 'dnf' ? (technical ? '+' : '\\lor') : technical ? '\\cdot' : '\\land';
  const literals = (pattern: string) => [...pattern].filter((c) => c !== '-').length;
  return (
    <span className="section-result logic-terms">
      <Katex latex={displayStyle(`${name} =`)} />
      {line.terms.map((k, i) => {
        const term = form.terms[k];
        const shared = term.outputs.length > 1;
        const latex = which === 'knf' && line.terms.length > 1 && literals(term.pattern) > 1 ? `(${term.latex})` : term.latex;
        return (
          <span key={k} className="logic-terms-part">
            {i > 0 && <Katex latex={displayStyle(join)} />}
            <span
              className={['logic-term', shared && 'shared', hover === k && 'active'].filter(Boolean).join(' ')}
              style={{ ['--block' as string]: `var(--geometry-${k % 8})` }}
              title={shared ? t('logic.joint.shared', { outputs: term.outputs.map((o) => form.outputs[o].name).join(', ') }) : undefined}
              onMouseEnter={() => onHover(k)}
              onMouseLeave={() => onHover(null)}
            >
              <Katex latex={displayStyle(latex)} />
            </span>
          </span>
        );
      })}
    </span>
  );
}

/**
 * The joint minimisation of all outputs (design/digital-logic.md §6.4.1): the joint minimal DNF
 * or KNF of each output with the shared terms marked, the cost against each output alone, the
 * choice among several solutions (which also drives the circuit drawn for all outputs), and the
 * steps.
 */
export function LogicJoint({ joint, scene, notation, hideSteps, onChange }: { joint: JointResult; scene: LogicScene; notation: LogicNotation; hideSteps: boolean; onChange(scene: LogicScene): void }) {
  const { t } = useT();
  const [which, setWhich] = useState<'dnf' | 'knf'>('dnf');
  const [hover, setHover] = useState<number | null>(null);
  const forms = joint[which];
  const form = forms.forms[forms.chosen];
  const key = which === 'dnf' ? 'jointChosen' : 'jointChosenKnf';

  const switches = (
    <Segmented
      choices={[
        { value: 'dnf', label: t('logic.joint.dnf') },
        { value: 'knf', label: t('logic.joint.knf') },
      ]}
      value={which}
      onChange={(w) => {
        setWhich(w);
        setHover(null);
      }}
    />
  );
  const meta = form
    ? [
        t('logic.joint.cost', { terms: form.cost.terms, literals: form.cost.literals, joins: form.cost.joins }),
        forms.separate ? t('logic.joint.separate', { terms: forms.separate.terms, literals: forms.separate.literals }) : '',
      ]
        .filter(Boolean)
        .join(' · ')
    : undefined;
  const choice =
    forms.forms.length > 1 ? (
      <div className="form-row logic-choices" title={t('logic.joint.choice.hint')}>
        <span className="logic-result-meta">{t('logic.minimal.choice', { count: forms.forms.length })}</span>
        {forms.forms.map((_, k) => (
          <button key={k} className={k === forms.chosen ? 'form-chip active' : 'form-chip'} onClick={() => onChange({ ...scene, [key]: k })}>
            {k + 1}
          </button>
        ))}
      </div>
    ) : undefined;

  return (
    <Section
      title="logic.joint.title"
      formula={
        <>
          {switches}
          {forms.error && <span className="section-message">{forms.error}</span>}
          {form && (
            <div className="logic-joint-lines">
              {form.outputs.map((_, o) => (
                <JointLine key={o} form={form} output={o} which={which} notation={notation} hover={hover} onHover={setHover} />
              ))}
            </div>
          )}
        </>
      }
      lines={[]}
      meta={meta}
      choice={choice}
      worked={forms.worked}
      hideSteps={hideSteps}
    >
      {joint.skipped.length > 0 && <p className="logic-result-meta">{t('logic.joint.skipped', { outputs: joint.skipped.join(', ') })}</p>}
    </Section>
  );
}
