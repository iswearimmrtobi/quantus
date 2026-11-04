import type { Geometry, GeometryKind, GeometryMeasure, GeometryTask, PlaneForm } from '../api';
import { useT } from '../i18n';
import type { TextKey } from '../i18n/en';
import { taskOfKind, TASK_KINDS, type TaskKind } from '../geometry';
import { useState } from 'react';
import { copyText } from '../shell';
import { Steps } from './AnalysisView';
import { displayStyle, Katex } from './Katex';
import { MathField } from './MathField';

type Built = Geometry['objects'];

/** The objects each slot of a task takes; the engine still checks the pair. */
const ANY: GeometryKind[] = ['point', 'vector', 'line', 'plane', 'sphere', 'circle'];
const SLOTS: Record<Exclude<TaskKind, 'convert' | 'measure' | 'evaluate'>, [GeometryKind[], GeometryKind[]]> = {
  relation: [ANY, ANY],
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

/** What can be measured of each kind of object. */
const MEASURES: Partial<Record<GeometryKind, GeometryMeasure[]>> = {
  vector: ['length'],
  sphere: ['surface', 'volume'],
  circle: ['circumference', 'area'],
};

const FORMS: PlaneForm[] = ['parameter', 'normal', 'coordinate'];

/** How a task names an object: a vector as `\vec{a}`, which keeps it apart from a point a. */
function reference(object: Built[number]): string {
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
      const [first, second] = SLOTS[task.kind];
      return (
        <>
          <Pick value={task.a} kinds={first} built={built} onChange={(a) => onChange({ ...task, a })} />
          <Pick value={task.b} kinds={second} built={built} onChange={(b) => onChange({ ...task, b })} />
        </>
      );
    }
    case 'foot':
      return (
        <>
          <Pick value={task.point} kinds={SLOTS.foot[0]} built={built} onChange={(point) => onChange({ ...task, point })} />
          <span className="geometry-task-word">{t('geometry.task.onto')}</span>
          <Pick value={task.onto} kinds={SLOTS.foot[1]} built={built} onChange={(onto) => onChange({ ...task, onto })} />
        </>
      );
    case 'reflect':
      return (
        <>
          <Pick value={task.point} kinds={SLOTS.reflect[0]} built={built} onChange={(point) => onChange({ ...task, point })} />
          <span className="geometry-task-word">{t('geometry.task.in')}</span>
          <Pick value={task.mirror} kinds={SLOTS.reflect[1]} built={built} onChange={(mirror) => onChange({ ...task, mirror })} />
        </>
      );
    case 'tangent':
      return (
        <>
          <Pick value={task.object} kinds={SLOTS.tangent[0]} built={built} onChange={(object) => onChange({ ...task, object })} />
          <span className="geometry-task-word">{t('geometry.task.at')}</span>
          <Pick value={task.point} kinds={SLOTS.tangent[1]} built={built} onChange={(point) => onChange({ ...task, point })} />
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
}

/** The task on the scene: what is asked, about which objects, and the exact answer with its steps. */
export function GeometryTaskPanel({ task, built, answer, onChange, hideSteps }: Props) {
  const { t } = useT();
  const [copied, setCopied] = useState(false);
  const copyAll = async () => {
    if (!answer?.alignBlock) return;
    await copyText(answer.alignBlock);
    setCopied(true);
    window.setTimeout(() => setCopied(false), 1200);
  };
  return (
    <section className="section-card geometry-task">
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
              <button className="copy-btn" title={t('linear.copy.hint')} onClick={() => void copyAll()}>
                {copied ? t('copied') : t('copy')}
              </button>
            )}
          </div>
          <Steps steps={answer.steps} hidden={hideSteps} display />
        </div>
      )}
    </section>
  );
}
