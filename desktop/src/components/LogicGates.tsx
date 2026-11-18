import { useEffect, useMemo, useRef, useState } from 'react';
import type { DrawForm, GateFormResult, LogicForm, LogicOutputResult, LogicScene, LogicSymbols, Worked } from '../api';
import { useT, type TextKey } from '../i18n';
import { logicTask } from '../logic';
import { fromGenerated, type GateChoice, type LogicPart } from '../logicCircuit';
import { staticPicture } from './LogicEditor';
import { Section } from './LogicResults';
import { Segmented } from './SettingsDialog';

/** The six forms of the switch, in its order. */
const FORMS: { form: DrawForm; label: TextKey; hint?: TextKey }[] = [
  { form: 'dnf', label: 'logic.gates.andOr' },
  { form: 'knf', label: 'logic.gates.orAnd' },
  { form: 'nand', label: 'logic.gates.nand' },
  { form: 'nand2', label: 'logic.gates.nand2', hint: 'logic.gates.twoInput.hint' },
  { form: 'nor', label: 'logic.gates.nor' },
  { form: 'nor2', label: 'logic.gates.nor2', hint: 'logic.gates.twoInput.hint' },
];

const KIND: Partial<Record<DrawForm, { kind: 'nand' | 'nor'; twoInput: boolean }>> = {
  nand: { kind: 'nand', twoInput: false },
  nand2: { kind: 'nand', twoInput: true },
  nor: { kind: 'nor', twoInput: false },
  nor2: { kind: 'nor', twoInput: true },
};

/** Whether `form` is a NAND or NOR form (one that can be rewritten from the expression). */
export const isGateForm = (form: DrawForm) => KIND[form] !== undefined;

/**
 * The NAND or NOR form `form` of `result` among its gate forms: the one of any width, or the
 * 2-input one. A form with no wider gate comes once, and stands for both.
 */
export function gateFormOf(result: LogicOutputResult, form: DrawForm, fromExpression: boolean): GateFormResult | undefined {
  const wanted = KIND[form];
  if (!wanted) return undefined;
  const origin = fromExpression ? 'rewrite' : 'minimal';
  const forms = result.gateForms.filter((f) => f.kind === wanted.kind && f.origin === origin);
  return wanted.twoInput ? forms.find((f) => f.twoInput) : forms[0];
}

/** Whether the 2-input form of `kind` differs from the one of any width, so that it gets a button of its own. */
const splits = (result: LogicOutputResult, kind: 'nand' | 'nor', fromExpression: boolean) =>
  result.gateForms.filter((f) => f.kind === kind && f.origin === (fromExpression ? 'rewrite' : 'minimal')).length > 1;

/** The choice as it can be shown: a 2-input form without one of its own falls back to the one of any width, as does a rewrite without an expression. */
export function shownChoice(result: LogicOutputResult, choice: GateChoice | undefined): { form: DrawForm; fromExpression: boolean } {
  let form = choice?.form ?? 'nand';
  const rewrites = result.gateForms.some((f) => f.origin === 'rewrite');
  const fromExpression = Boolean(choice?.fromExpression) && rewrites && isGateForm(form);
  if (form === 'nand2' && !splits(result, 'nand', fromExpression)) form = 'nand';
  if (form === 'nor2' && !splits(result, 'nor', fromExpression)) form = 'nor';
  if ((form === 'dnf' || form === 'knf') && !result.minimal) form = 'nand';
  return { form, fromExpression };
}

/**
 * The Gates card (ux-rework.md 1.4): one output's function as AND–OR, OR–AND, NAND, NAND of
 * 2 inputs, NOR or NOR of 2 inputs, with the formula, its cost, Copy and the steps. The NAND and
 * NOR forms come from the minimal form or, when the output has an expression, from that.
 */
export function LogicGates({
  scene,
  result,
  symbols,
  choice,
  onChoose,
  onOpen,
  eq,
  cost,
  gates,
  hideSteps,
}: {
  scene: LogicScene;
  result: LogicOutputResult;
  symbols: LogicSymbols;
  choice: GateChoice | undefined;
  onChoose(choice: GateChoice): void;
  /** **Open in editor**: the circuit of the shown form drawn in the editor. */
  onOpen(form: DrawForm, fromExpression: boolean): void;
  eq(latex: string): string;
  cost(form: LogicForm): string;
  gates(form: GateFormResult): string;
  hideSteps: boolean;
}) {
  const { t } = useT();
  const { form, fromExpression } = shownChoice(result, choice);
  const { minimal } = result;
  const rewrites = result.gateForms.some((f) => f.origin === 'rewrite');

  const available = FORMS.filter(({ form: f }) => {
    if (f === 'dnf' || f === 'knf') return minimal !== null;
    if (f === 'nand2') return splits(result, 'nand', fromExpression);
    if (f === 'nor2') return splits(result, 'nor', fromExpression);
    return gateFormOf(result, f, fromExpression) !== undefined;
  });
  if (!available.length) return null;

  let latex = '';
  let meta = '';
  let worked: Worked | null = null;
  if (form === 'dnf' || form === 'knf') {
    const two = form === 'dnf' ? minimal?.dnf[minimal.chosen] : minimal?.knf[minimal.chosenKnf];
    if (two && minimal) {
      latex = two.latex;
      meta = cost(two);
      worked = form === 'dnf' ? minimal.worked : minimal.knfWorked;
    }
  } else {
    const gate = gateFormOf(result, form, fromExpression);
    if (gate) {
      latex = gate.latex;
      meta = gates(gate);
      worked = gate.worked;
    }
  }

  const switches = (
    <div className="logic-gates-switches">
      <Segmented
        choices={available.map(({ form: f, label, hint }) => ({ value: f, label: t(label), hint: hint && t(hint) }))}
        value={form}
        onChange={(f) => onChoose({ form: f, fromExpression })}
      />
      {rewrites && isGateForm(form) && (
        <Segmented
          choices={[
            { value: 'minimal', label: t('logic.gates.fromMinimal') },
            { value: 'expression', label: t('logic.gates.fromExpression') },
          ]}
          value={fromExpression ? 'expression' : 'minimal'}
          onChange={(v) => onChoose({ form, fromExpression: v === 'expression' })}
        />
      )}
    </div>
  );

  return (
    <Section title="logic.gates.title" formula={switches} lines={latex ? [eq(latex)] : []} meta={meta} worked={worked} hideSteps={hideSteps}>
      {latex && (
        <div className="logic-gates-circuit">
          <GatesPicture scene={scene} output={result.name} form={form} fromExpression={fromExpression} symbols={symbols} />
          <button className="form-chip" title={t('logic.draw.hint')} onClick={() => onOpen(form, fromExpression)}>
            {t('logic.gates.open')}
          </button>
        </div>
      )}
    </Section>
  );
}

/**
 * The circuit of `form` of output `output`, drawn by the core as **Draw from** draws it, shown
 * read-only in the app's colours. It is drawn again when the scene's functions or the form change.
 */
function GatesPicture({ scene, output, form, fromExpression, symbols }: { scene: LogicScene; output: string; form: DrawForm; fromExpression: boolean; symbols: LogicSymbols }) {
  const [parts, setParts] = useState<LogicPart[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  /** The latest request; an answer to an older one is dropped. */
  const request = useRef(0);
  // What the drawing depends on: the functions and the circuit (a rewrite of a traced expression), not the editor's data.
  const key = JSON.stringify([scene.inputs, scene.outputs, scene.circuit, output, form, fromExpression]);
  const latest = useRef(scene);
  latest.current = scene;
  useEffect(() => {
    const id = ++request.current;
    void logicTask(latest.current, { kind: 'draw', form, output, fromExpression }).then(({ circuit, error }) => {
      if (id !== request.current) return;
      setParts(circuit ? fromGenerated(circuit) : null);
      setError(error);
    });
  }, [key]);
  const picture = useMemo(() => (parts?.length ? staticPicture(parts, symbols, 'app') : null), [parts, symbols]);
  if (error) return <p className="file-error">{error}</p>;
  if (!picture) return null;
  return (
    <img
      className="logic-gates-picture"
      alt=""
      width={picture.width}
      height={picture.height}
      src={`data:image/svg+xml;charset=utf-8,${encodeURIComponent(picture.svg)}`}
    />
  );
}
