import { useEffect, useRef, useState } from 'react';
import { ApiError, type LogicOutput, type LogicScene, type LogicSource } from '../api';
import { isLine, lineSource, newLine, type LogicAnswer } from '../logic';
import { useT } from '../i18n';
import { ErrorRow } from './AnalysisView';
import { InsertBar, type InsertKey } from './InsertBar';
import { MathField, type MathFieldHandle } from './MathField';

interface Props {
  scene: LogicScene;
  answer: LogicAnswer | null;
  onChange(scene: LogicScene): void;
}

/** Which view an output's function comes from (§5.1): a small mark beside its name. */
export function SourceMark({ kind }: { kind: LogicSource['kind'] }) {
  const { t } = useT();
  return (
    <span className={`logic-source logic-source-${kind}`} title={t('logic.source.hint')}>
      {t(`logic.source.${kind}`)}
    </span>
  );
}

/** The operators a keyboard lacks, put in at the cursor of the line typed in last. */
const KEYS: InsertKey[] = [
  { label: '\\land', latex: ' \\land ', math: true },
  { label: '\\lor', latex: ' \\lor ', math: true },
  { label: '\\lnot', latex: '\\lnot ', math: true },
  { label: '\\oplus', latex: ' \\oplus ', math: true },
  { label: '(', latex: '(', math: true },
  { label: ')', latex: ')', math: true },
  { label: '\\Sigma m', latex: '\\sum m\\left(#?\\right)', math: true },
];

/** The example in the first empty line. */
const PLACEHOLDER = 'f = A \\land B \\lor C';

/**
 * One function line per output (§5.2, §5.3): an expression or a minterm list, typed in MathLive.
 * A line's output takes it as its source as it is typed; an output from the table or the circuit
 * keeps that source until Enter, so that a stray key does not throw a filled table away. Enter
 * moves to the next line (**+ Function** adds one), Backspace in an empty line removes it, Up and
 * Down move between lines. Each line's errors stand under it.
 */
export function LogicLines({ scene, answer, onChange }: Props) {
  const { t } = useT();
  /** Stable keys for the lines, so a removed line takes its own field with it. */
  const ids = useRef<number[]>([]);
  const nextId = useRef(0);
  while (ids.current.length < scene.outputs.length) ids.current.push(nextId.current++);
  ids.current.length = scene.outputs.length;
  const fields = useRef(new Map<number, MathFieldHandle>());
  /** The line that takes the focus once it appears. */
  const [focusId, setFocusId] = useState<number | null>(null);
  useEffect(() => {
    if (focusId !== null) setFocusId(null);
  }, [focusId]);
  /** The line typed in last, where the operator keys put their text. */
  const lastFocused = useRef<number | null>(null);
  /** What is typed over an output from the table or the circuit, by line id, until Enter. */
  const [drafts, setDrafts] = useState<Record<number, string>>({});

  const outputs = scene.outputs;
  const setOutputs = (next: LogicOutput[]) => onChange({ ...scene, outputs: next });
  const focus = (index: number) => {
    const id = ids.current[index];
    if (id !== undefined) fields.current.get(id)?.focus();
  };

  const change = (index: number, latex: string) => {
    const output = outputs[index];
    if (isLine(output)) setOutputs(outputs.map((o, i) => (i === index ? { ...o, source: lineSource(latex) } : o)));
    else setDrafts({ ...drafts, [ids.current[index]]: latex });
  };

  const enter = (index: number) => {
    const id = ids.current[index];
    const draft = drafts[id];
    let next = outputs;
    if (draft !== undefined && draft.trim()) {
      // The typed line replaces the table or the circuit as the output's source.
      next = outputs.map((o, i) => (i === index ? { ...o, source: lineSource(draft) } : o));
      const rest = { ...drafts };
      delete rest[id];
      setDrafts(rest);
      // The field is made anew as a line's; it keeps the focus.
      setFocusId(id);
    }
    // On to the next line; a new line comes only from **+ Function**.
    if (index < next.length - 1) focus(index + 1);
    if (next !== outputs) setOutputs(next);
  };

  const remove = (index: number) => {
    if (outputs.length === 1) {
      setOutputs([newLine()]);
      return;
    }
    ids.current.splice(index, 1);
    setOutputs(outputs.filter((_, i) => i !== index));
    if (index > 0) setFocusId(ids.current[index - 1]);
  };

  const add = () => {
    setFocusId(nextId.current);
    setOutputs([...outputs, newLine()]);
  };

  const insert = (latex: string) => {
    const id = lastFocused.current !== null && ids.current.includes(lastFocused.current) ? lastFocused.current : ids.current[0];
    if (id !== undefined) fields.current.get(id)?.insert(latex);
  };

  return (
    <section className="section-card logic-lines">
      <InsertBar keys={KEYS} onInsert={insert} />
      {outputs.map((output, index) => {
        const id = ids.current[index];
        const result = answer?.outputs[index] ?? null;
        const errors = answer?.errors.filter((error) => error.output === index) ?? [];
        const line = isLine(output);
        // A line that names its output (`f = …`, `f(A, B) = …`) needs no name beside it.
        const name = line && output.source.latex.includes('=') ? '' : (result?.name ?? output.name ?? '');
        const mark = Boolean(result) || !line;
        return (
          <div className={errors.length ? 'logic-line has-error' : 'logic-line'} key={id}>
            <div className="logic-line-row" onFocus={() => (lastFocused.current = id)}>
              {(name || mark) && (
                <span className="logic-line-name">
                  {name && <span className="logic-name">{name}</span>}
                  {mark && <SourceMark kind={output.source.kind} />}
                </span>
              )}
              <MathField
                // A new field when the output starts or stops being a line, with its own placeholder.
                key={line ? 'line' : 'other'}
                ref={(handle) => {
                  if (handle) fields.current.set(id, handle);
                  else fields.current.delete(id);
                }}
                value={line ? output.source.latex : (drafts[id] ?? '')}
                onChange={(latex) => change(index, latex)}
                onEnter={() => enter(index)}
                onBackspaceEmpty={line ? () => remove(index) : undefined}
                onLeaveLine={(direction) => focus(index + direction)}
                placeholder={line ? (index === 0 ? PLACEHOLDER : undefined) : `\\text{${t(output.source.kind === 'table' ? 'logic.line.fromTable' : 'logic.line.fromCircuit')}}`}
                autoFocus={focusId === id || (index === 0 && outputs.length === 1 && line && !output.source.latex)}
              />
              <button className="logic-line-remove" title={t('logic.line.remove')} onClick={() => remove(index)}>
                ✕
              </button>
            </div>
            {errors.map((error, i) => (
              <ErrorRow key={i} error={new ApiError(error.message)} soft={line} />
            ))}
          </div>
        );
      })}
      <button className="palette-item logic-line-add" onClick={add}>
        + {t('logic.line.add')}
      </button>
    </section>
  );
}
