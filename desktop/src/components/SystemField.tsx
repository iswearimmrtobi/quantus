import { useRef, useState } from 'react';
import { useT } from '../i18n';
import { LatexInput } from './LatexInput';
import { MathField, toReadableLatex, type MathFieldHandle } from './MathField';

const PLACEHOLDERS = ['2x + y = 5', 'x - y = 1'];

/**
 * A line break, written or typed: `\\` as in a `cases` block, or a newline. A pasted `cases`
 * block loses its frame first, so its rows become lines.
 */
function splitLines(text: string): string[] {
  const rows = text.replace(/\\begin\{cases\}|\\end\{cases\}/g, '');
  return rows.split(/\\\\|\r?\n/).map((line) => line.trim());
}

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
 * Enter solves; a second Enter right after it starts a line below, and so does a line break:
 * `\\` (in the formula editor the first `\` opens the source, the second one breaks the line) or
 * a newline, typed with Shift+Enter in the source or pasted. Backspace in an empty line removes
 * it, Up and Down move between lines. Esc goes back from the source to the formula.
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
  /** Whether the last key was an Enter, so that the next one starts a line. */
  const enterBefore = useRef(false);

  const focus = (index: number) => {
    const id = ids.current[index];
    if (id === undefined) return;
    setFocusId(id);
    fields.current.get(id)?.focus();
  };

  /**
   * Replaces line [index] by [parts], the first in its place and the others below it; the focus
   * goes to the last one, in the formula editor.
   */
  const replaceLine = (index: number, parts: string[]) => {
    const added = parts.slice(1).map(() => nextId.current++);
    ids.current.splice(index + 1, 0, ...added);
    setFocusId(added.at(-1) ?? ids.current[index]);
    setLatexId(null);
    onChange([...lines.slice(0, index), ...parts, ...lines.slice(index + 1)]);
    onSubmit();
  };

  const setLine = (index: number, value: string) => {
    const parts = splitLines(value);
    if (parts.length > 1) replaceLine(index, parts);
    else onChange(lines.map((line, i) => (i === index ? value : line)));
  };

  const enter = (index: number) => {
    if (enterBefore.current) {
      enterBefore.current = false;
      replaceLine(index, [lines[index], '']);
      return;
    }
    enterBefore.current = true;
    onSubmit();
  };

  /** A paste with line breaks fills lines from here on; one without is MathLive's. */
  const paste = (index: number, text: string) => {
    const parts = splitLines(text);
    if (parts.length < 2) return false;
    replaceLine(index, [lines[index] + parts[0], ...parts.slice(1)]);
    return true;
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
    <div
      className={lines.length > 1 ? 'system-field multiline' : 'system-field'}
      onKeyDownCapture={(event) => {
        if (event.key !== 'Enter' && !['Shift', 'Control', 'Alt', 'Meta'].includes(event.key)) enterBefore.current = false;
      }}
    >
      {lines.map((line, index) => {
        const id = ids.current[index];
        return (
          <div className={latexId === id ? 'input-row system-line latex' : 'input-row system-line'} key={id}>
            {latexId === id ? (
              <LatexInput
                value={line}
                onChange={(value) => setLine(index, value)}
                onEnter={() => enter(index)}
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
                onChange={(value) => onChange(lines.map((other, i) => (i === index ? value : other)))}
                onEnter={() => enter(index)}
                onLatexRequested={(seed) => toggleLatex(id, index, seed)}
                onBackspaceEmpty={() => removeLine(index)}
                onLeaveLine={(direction) => focus(index + direction)}
                onPasteText={(text) => paste(index, text)}
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
