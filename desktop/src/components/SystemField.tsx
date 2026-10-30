import { useRef, useState } from 'react';
import { useT } from '../i18n';
import { LatexInput } from './LatexInput';
import { MathField, toReadableLatex, type MathFieldHandle } from './MathField';

const PLACEHOLDERS = ['2x + y = 5', 'x - y = 1'];

interface Props {
  lines: string[];
  onChange(lines: string[]): void;
  /** Solve what is there; called after every Enter. */
  onSubmit(): void;
}

/**
 * A system typed one equation per line: a formula field for each line, drawn as one block with a
 * brace. One field with a `cases` block would read the same, but MathLive loses the block on
 * Home/End + Backspace and cannot remove an empty line (see the spike in plans/linear-systems.md).
 *
 * Enter starts a line below and solves, Backspace in an empty line removes it, Up and Down move
 * between lines. `\` writes the line as LaTeX source, Esc goes back.
 */
export function SystemField({ lines, onChange, onSubmit }: Props) {
  const { t } = useT();
  const ctrl = t('key.ctrl');
  /** Stable keys for the lines, so a removed line takes its own field with it. */
  const ids = useRef<number[]>([]);
  const nextId = useRef(0);
  while (ids.current.length < lines.length) ids.current.push(nextId.current++);
  ids.current.length = lines.length;
  const fields = useRef(new Map<number, MathFieldHandle>());
  /** The line that takes the focus when it appears; the first one when the page opens. */
  const [focusId, setFocusId] = useState<number | null>(ids.current[0] ?? null);
  /** The line written as LaTeX source, by id. */
  const [latexId, setLatexId] = useState<number | null>(null);

  const focus = (index: number) => {
    const id = ids.current[index];
    if (id === undefined) return;
    setFocusId(id);
    fields.current.get(id)?.focus();
  };

  const setLine = (index: number, value: string) => onChange(lines.map((line, i) => (i === index ? value : line)));

  const addLineAfter = (index: number) => {
    const id = nextId.current++;
    ids.current.splice(index + 1, 0, id);
    setFocusId(id);
    setLatexId(null);
    onChange([...lines.slice(0, index + 1), '', ...lines.slice(index + 1)]);
    onSubmit();
  };

  const removeLine = (index: number) => {
    if (lines.length === 1) return;
    ids.current.splice(index, 1);
    onChange(lines.filter((_, i) => i !== index));
    focus(Math.max(0, index - 1));
  };

  const toggleLatex = (id: number, index: number, seed?: string) => {
    if (latexId === id) {
      setLatexId(null);
      setFocusId(id);
      return;
    }
    setLine(index, seed ?? toReadableLatex(lines[index]));
    setLatexId(id);
  };

  return (
    <div className={lines.length > 1 ? 'system-field multiline' : 'system-field'}>
      {lines.map((line, index) => {
        const id = ids.current[index];
        return (
          <div className={latexId === id ? 'input-row system-line latex' : 'input-row system-line'} key={id}>
            {latexId === id ? (
              <LatexInput
                value={line}
                onChange={(value) => setLine(index, value)}
                onEnter={() => addLineAfter(index)}
                onLeave={() => toggleLatex(id, index)}
                placeholder={PLACEHOLDERS[index] ?? ''}
              />
            ) : (
              <MathField
                ref={(handle) => {
                  if (handle) fields.current.set(id, handle);
                  else fields.current.delete(id);
                }}
                value={line}
                onChange={(value) => setLine(index, value)}
                onEnter={() => addLineAfter(index)}
                onLatexRequested={(seed) => toggleLatex(id, index, seed)}
                onBackspaceEmpty={() => removeLine(index)}
                onLeaveLine={(direction) => focus(index + direction)}
                placeholder={PLACEHOLDERS[index]}
                autoFocus={id === focusId}
              />
            )}
            <button
              className="mode-toggle"
              title={latexId === id ? t('editor.back') : t('editor.latex', { ctrl })}
              onClick={() => toggleLatex(id, index)}
            >
              {latexId === id ? 'TeX' : 'f(x)'}
            </button>
          </div>
        );
      })}
    </div>
  );
}
