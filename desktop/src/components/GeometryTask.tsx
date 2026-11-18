import type { Geometry, GeometryKind, GeometryMeasure, GeometryTask, PlaneForm, Step } from '../api';
import { useT } from '../i18n';
import type { TextKey } from '../i18n';
import { taskOfKind, TASK_KINDS, type TaskKind } from '../geometry';
import { useState } from 'react';
import { copyText } from '../shell';
import { Steps } from './AnalysisView';
import { displayStyle, Katex } from './Katex';
import { MathField } from './MathField';

type Built = Geometry['objects'];

type PairTask = Exclude<TaskKind, 'convert' | 'measure' | 'evaluate'>;

/** The objects each slot of a task takes; `PAIRS` says which of them go together. */
const RELATED: GeometryKind[] = ['point', 'line', 'plane', 'sphere', 'circle'];
const SLOTS: Record<PairTask, [GeometryKind[], GeometryKind[]]> = {
  relation: [RELATED, RELATED],
  distance: [
    ['point', 'line', 'plane', 'sphere'],
    ['point', 'line', 'plane', 'sphere'],
  ],
  angle: [
    ['vector', 'line', 'plane'],
    ['vector', 'line', 'plane'],
  ],
  midpoint: [['point'], ['point']],
  foot: [['point'], ['line', 'plane']],
  reflect: [['point'], ['point', 'line', 'plane']],
  tangent: [['sphere', 'circle'], ['point']],
};

/**
 * The pairs of kinds each task takes, as `quantus_core::geometry::interface` answers them, in
 * the order of its slots. The tasks in `SYMMETRIC` also take each pair the other way round.
 */
const PAIRS: Record<PairTask, string[]> = {
  relation: ['line-line', 'line-plane', 'plane-plane', 'point-line', 'point-plane', 'point-sphere', 'point-circle', 'sphere-line', 'sphere-plane', 'sphere-sphere', 'circle-line', 'circle-plane'],
  distance: ['point-point', 'point-line', 'point-plane', 'point-sphere', 'line-line', 'line-plane', 'plane-plane'],
  angle: ['vector-vector', 'line-line', 'line-plane', 'plane-plane'],
  midpoint: ['point-point'],
  foot: ['point-line', 'point-plane'],
  reflect: ['point-point', 'point-line', 'point-plane'],
  tangent: ['sphere-point', 'circle-point'],
};
const SYMMETRIC: PairTask[] = ['relation', 'distance', 'angle', 'midpoint'];

/** Whether `task` takes an object of kind `a` in its first slot and one of kind `b` in its second. */
function fits(task: PairTask, a: string, b: string): boolean {
  return PAIRS[task].includes(`${a}-${b}`) || (SYMMETRIC.includes(task) && PAIRS[task].includes(`${b}-${a}`));
}

/** The kinds of the second slot that go with the object in the first, or all of the slot's while it is empty. */
function secondKinds(task: PairTask, first: string, built: Built): GeometryKind[] {
  const kind = built.find((object) => !object.error && reference(object) === first)?.kind;
  return kind ? SLOTS[task][1].filter((other) => fits(task, kind, other)) : SLOTS[task][1];
}

/** The task of `kind` on the objects `a` and `b`, by the names a task uses. */
function pairTask(kind: PairTask, a: string, b: string): GeometryTask {
  switch (kind) {
    case 'foot':
      return { kind, point: a, onto: b };
    case 'reflect':
      return { kind, point: a, mirror: b };
    case 'tangent':
      return { kind, object: a, point: b };
    default:
      return { kind, a, b };
  }
}

/**
 * The tasks that fit the selected objects, in the panel's order: for one object what can be
 * measured or converted of it, for two each task that takes them, in whichever order it does.
 */
export function fittingTasks(selected: Built): GeometryTask[] {
  const objects = selected.filter((object) => !object.error);
  if (objects.length === 1) {
    const [object] = objects;
    const measures = MEASURES[object.kind as GeometryKind] ?? [];
    const tasks: GeometryTask[] = [];
    if (object.kind === 'plane') tasks.push({ kind: 'convert', plane: reference(object), form: 'coordinate' });
    if (measures.length) tasks.push({ kind: 'measure', object: reference(object), measure: measures[0] });
    return tasks;
  }
  if (objects.length !== 2) return [];
  const [a, b] = objects;
  const tasks: GeometryTask[] = [];
  for (const kind of TASK_KINDS) {
    if (!(kind in PAIRS)) continue;
    const task = kind as PairTask;
    if (fits(task, a.kind, b.kind)) tasks.push(pairTask(task, reference(a), reference(b)));
    else if (fits(task, b.kind, a.kind)) tasks.push(pairTask(task, reference(b), reference(a)));
  }
  return tasks;
}

/** The steps without the result lines they end with, which the answer shows above them already. */
export function stepsBelow(steps: Step[], result: string[]): Step[] {
  let end = steps.length;
  while (end > 0 && result.includes(steps[end - 1].latex)) end--;
  return steps.slice(0, end);
}

/** What can be measured of each kind of object. */
const MEASURES: Partial<Record<GeometryKind, GeometryMeasure[]>> = {
  vector: ['length'],
  sphere: ['surface', 'volume'],
  circle: ['circumference', 'area'],
};

const FORMS: PlaneForm[] = ['parameter', 'normal', 'coordinate'];

/** How a task names an object: a vector as `\vec{a}`, which keeps it apart from a point a. */
export function reference(object: Built[number]): string {
  if (object.kind !== 'vector') return object.name;
  const bare = object.name.replace(/^\\vec\s*/, '').replace(/[{}\s]/g, '');
  return `\\vec{${bare}}`;
}

/** A pick of an object of the scene among those of `kinds` that were built. */
function Pick({ value, kinds, built, onChange }: { value: string; kinds: GeometryKind[]; built: Built; onChange(value: string): void }) {
  const { t } = useT();
  const choices = built.filter((object) => !object.error && kinds.includes(object.kind as GeometryKind));
  const values = choices.map(reference);
  return (
    <select className="setting-select geometry-pick" value={value} onChange={(event) => onChange(event.target.value)}>
      <option value="">{t('geometry.pick')}</option>
      {value && !values.includes(value) && <option value={value}>{value}</option>}
      {choices.map((object, i) => (
        <option key={values[i]} value={values[i]}>
          {object.kind === 'vector' ? `${values[i].slice(5, -1)}⃗` : object.name}
        </option>
      ))}
    </select>
  );
}

/** The fields of the task, by its kind. */
function Slots({ task, built, onChange }: { task: GeometryTask; built: Built; onChange(task: GeometryTask): void }) {
  const { t } = useT();
  switch (task.kind) {
    case 'relation':
    case 'distance':
    case 'angle':
    case 'midpoint': {
      return (
        <>
          <Pick value={task.a} kinds={SLOTS[task.kind][0]} built={built} onChange={(a) => onChange({ ...task, a })} />
          <Pick value={task.b} kinds={secondKinds(task.kind, task.a, built)} built={built} onChange={(b) => onChange({ ...task, b })} />
        </>
      );
    }
    case 'foot':
      return (
        <>
          <Pick value={task.point} kinds={SLOTS.foot[0]} built={built} onChange={(point) => onChange({ ...task, point })} />
          <span className="geometry-task-word">{t('geometry.task.onto')}</span>
          <Pick value={task.onto} kinds={secondKinds('foot', task.point, built)} built={built} onChange={(onto) => onChange({ ...task, onto })} />
        </>
      );
    case 'reflect':
      return (
        <>
          <Pick value={task.point} kinds={SLOTS.reflect[0]} built={built} onChange={(point) => onChange({ ...task, point })} />
          <span className="geometry-task-word">{t('geometry.task.in')}</span>
          <Pick value={task.mirror} kinds={secondKinds('reflect', task.point, built)} built={built} onChange={(mirror) => onChange({ ...task, mirror })} />
        </>
      );
    case 'tangent':
      return (
        <>
          <Pick value={task.object} kinds={SLOTS.tangent[0]} built={built} onChange={(object) => onChange({ ...task, object })} />
          <span className="geometry-task-word">{t('geometry.task.at')}</span>
          <Pick value={task.point} kinds={secondKinds('tangent', task.object, built)} built={built} onChange={(point) => onChange({ ...task, point })} />
        </>
      );
    case 'convert':
      return (
        <>
          <Pick value={task.plane} kinds={['plane']} built={built} onChange={(plane) => onChange({ ...task, plane })} />
          <span className="geometry-task-word">{t('geometry.task.into')}</span>
          <select className="setting-select geometry-pick" value={task.form} onChange={(event) => onChange({ ...task, form: event.target.value as PlaneForm })}>
            {FORMS.map((form) => (
              <option key={form} value={form}>
                {t(`geometry.form.${form}` as TextKey)}
              </option>
            ))}
          </select>
        </>
      );
    case 'measure': {
      const object = built.find((o) => reference(o) === task.object);
      const measures = (object && MEASURES[object.kind as GeometryKind]) ?? [];
      return (
        <>
          <Pick
            value={task.object}
            kinds={Object.keys(MEASURES) as GeometryKind[]}
            built={built}
            onChange={(name) => {
              // The first measure of the new object's kind, if the old one does not fit it.
              const kind = built.find((o) => reference(o) === name)?.kind as GeometryKind | undefined;
              const fits = kind ? (MEASURES[kind] ?? []) : [];
              onChange({ ...task, object: name, measure: fits.includes(task.measure) ? task.measure : (fits[0] ?? task.measure) });
            }}
          />
          {measures.length > 1 && (
            <select className="setting-select geometry-pick" value={task.measure} onChange={(event) => onChange({ ...task, measure: event.target.value as GeometryMeasure })}>
              {measures.map((measure) => (
                <option key={measure} value={measure}>
                  {t(`geometry.measure.${measure}` as TextKey)}
                </option>
              ))}
            </select>
          )}
        </>
      );
    }
    case 'evaluate':
      return (
        <span className="matrix-cell geometry-cell line" title={t('geometry.task.evaluate.hint')}>
          <MathField className="cell-input" value={task.latex} onChange={(latex) => onChange({ ...task, latex })} onEnter={() => {}} autoFocus={false} />
        </span>
      );
  }
}

interface Props {
  task: GeometryTask | null;
  built: Built | null;
  /** The engine's answer to the task. */
  answer: Geometry['task'];
  onChange(task: GeometryTask | null): void;
  /** At the detail level `none` the steps are hidden. */
  hideSteps: boolean;
  /** The objects selected in the sidebar or the view, whose fitting tasks are offered. */
  selected?: Built;
}

/** The task on the scene: what is asked, about which objects, and the exact answer with its steps. */
export function GeometryTaskPanel({ task, built, answer, onChange, hideSteps, selected = [] }: Props) {
  const { t } = useT();
  const [copied, setCopied] = useState(false);
  const suggestions = fittingTasks(selected);
  const copyAll = async () => {
    if (!answer?.alignBlock) return;
    await copyText(answer.alignBlock);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1200);
  };
  return (
    <section className="section-card geometry-task">
      {suggestions.length > 0 && (
        <div className="geometry-suggestions">
          <span className="geometry-task-word">{t('geometry.task.fitting')}</span>
          {suggestions.map((suggestion) => (
            <button key={JSON.stringify(suggestion)} className="palette-item" onClick={() => onChange(suggestion)}>
              {t(`geometry.task.${suggestion.kind}` as TextKey)}
            </button>
          ))}
        </div>
      )}
      <div className="geometry-task-form">
        <select
          className="setting-select geometry-pick"
          value={task?.kind ?? ''}
          onChange={(event) => onChange(event.target.value ? taskOfKind(event.target.value as TaskKind, task) : null)}
        >
          <option value="">{t('geometry.task.choose')}</option>
          {TASK_KINDS.map((kind) => (
            <option key={kind} value={kind}>
              {t(`geometry.task.${kind}` as TextKey)}
            </option>
          ))}
        </select>
        {task && <Slots task={task} built={built ?? []} onChange={onChange} />}
      </div>
      {answer?.error && <span className="geometry-error">{answer.error}</span>}
      {answer && !answer.error && (
        <div className="geometry-answer">
          <div className="linear-result-head">
            <div className="geometry-answer-lines">
              {answer.relation && <span className="geometry-relation">{t(`geometry.relation.${answer.relation}` as TextKey)}</span>}
              {answer.result.map((line, i) => (
                <Katex key={i} className="section-result" latex={displayStyle(line)} />
              ))}
            </div>
            {answer.alignBlock && (
              <button className="copy-btn" title={t('geometry.copy.hint')} onClick={() => void copyAll()}>
                {copied ? t('copied') : t('copy')}
              </button>
            )}
          </div>
          <Steps steps={stepsBelow(answer.steps, answer.result)} hidden={hideSteps} display />
        </div>
      )}
    </section>
  );
}
