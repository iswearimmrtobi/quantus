import { useRef, useState, type KeyboardEvent } from 'react';
import type { LogicScene, LogicValue } from '../api';
import {
  addInput,
  addOutput,
  cycle,
  dependsOn,
  editColumn,
  isLine,
  MAX_INPUTS,
  mentions,
  nameLatex,
  readName,
  removeInput,
  removeOutput,
  renameInput,
  renameOutputAt,
  type Cell,
  type Grid,
  type LogicAnswer,
} from '../logic';
import { useT } from '../i18n';
import { Katex } from './Katex';
import { CopyButton } from './LogicResults';

interface Props {
  scene: LogicScene;
  grid: Grid;
  answer: LogicAnswer | null;
  /** Whether 0s are written or left empty. */
  zeros: boolean;
  onChange(scene: LogicScene): void;
  /** The row the circuit's switches are set to, highlighted; null without a circuit. */
  currentRow?: number | null;
  /** Sets the switches to a row, by a click on its number. */
  onRow?(row: number): void;
}

/** A header named in place: a click opens a text field, Enter or leaving it renames, Esc keeps the name. */
function Header({ name, taken, onRename }: { name: string; taken: string[]; onRename?(name: string): void }) {
  const { t } = useT();
  const [text, setText] = useState<string | null>(null);
  if (text === null || !onRename) {
    return (
      <button className="logic-header-name" title={onRename ? t('logic.table.rename') : undefined} disabled={!onRename} onClick={() => setText(name)}>
        <Katex latex={nameLatex(name)} />
      </button>
    );
  }
  const read = readName(text);
  const valid = read !== null && (read === name || !taken.includes(read));
  const done = () => {
    if (valid && read !== name) onRename(read);
    setText(null);
  };
  return (
    <input
      className={valid ? 'logic-header-input' : 'logic-header-input invalid'}
      value={text}
      autoFocus
      size={Math.max(2, text.length)}
      title={valid ? undefined : t('logic.table.badName')}
      onChange={(event) => setText(event.target.value)}
      onBlur={done}
      onKeyDown={(event) => {
        if (event.key === 'Enter') done();
        else if (event.key === 'Escape') {
          event.stopPropagation();
          setText(null);
        }
      }}
    />
  );
}

/**
 * The truth table (§5.4): the input columns, a line, the output columns, rows in binary order with
 * the row number first. An output cell cycles 0 → 1 → – with a click or Space; the keys 0, 1 and
 * - set it and move down, the arrows move. Editing a cell makes the table its output's source.
 */
export function LogicTable({ scene, grid, answer, zeros, onChange, currentRow = null, onRow }: Props) {
  const { t } = useT();
  const table = useRef<HTMLTableElement>(null);
  /** The input whose removal waits for a yes, because a table depends on it. */
  const [asking, setAsking] = useState<number | null>(null);
  const { inputs, columns } = grid;
  const n = inputs.length;
  const rows = 1 << n;
  const names = [...inputs, ...columns.map((c) => c.name)];

  /** The rows of each output that an error is about. */
  const errorRows = new Map<number, Set<number>>();
  for (const error of answer?.errors ?? []) if (error.output !== null && error.rows.length) errorRows.set(error.output, new Set(error.rows));

  const focusCell = (row: number, col: number) => {
    const cell = table.current?.querySelector<HTMLButtonElement>(`[data-row="${row}"][data-col="${col}"]`);
    cell?.focus();
  };

  const set = (index: number, row: number, value: LogicValue) =>
    onChange(editColumn(scene, grid, index, (values) => values.map((v, r) => (r === row ? value : v))));

  const onKey = (event: KeyboardEvent, index: number, row: number, col: number, value: Cell) => {
    const key = event.key;
    if (key === '0' || key === '1' || key === '-') {
      event.preventDefault();
      set(index, row, key);
      focusCell(Math.min(row + 1, rows - 1), col);
    } else if (key === ' ' || key === 'Enter') {
      event.preventDefault();
      set(index, row, cycle(value));
    } else if (key === 'ArrowDown' || key === 'ArrowUp') {
      event.preventDefault();
      focusCell(Math.max(0, Math.min(rows - 1, row + (key === 'ArrowDown' ? 1 : -1))), col);
    } else if (key === 'ArrowLeft' || key === 'ArrowRight') {
      event.preventDefault();
      focusCell(row, col + (key === 'ArrowRight' ? 1 : -1));
    }
  };

  /** Removes input `k` at once, or asks first when a table depends on it. */
  const askRemove = (k: number) => {
    const needed = columns.some((c) => c.source === 'table' && dependsOn(c.values, n, k));
    if (needed && asking !== k) setAsking(k);
    else {
      setAsking(null);
      onChange(removeInput(scene, grid, k));
    }
  };

  const shown = (value: Cell) => (value === null ? '' : value === '-' ? '–' : value === '0' && !zeros ? '' : value);

  return (
    <section className="section-card logic-table-card">
      <div className="logic-table-scroll">
        <table className="logic-table" ref={table}>
          <thead>
            <tr>
              <th className="logic-row-number">
                <span title={t('logic.table.row')}>i</span>
              </th>
              {inputs.map((name, k) => {
                // A line that names the input keeps it: the line, not the table, has to change.
                const line = scene.outputs.find((o) => isLine(o) && mentions(o.source.latex, name));
                return (
                  <th key={name} className="logic-input-head">
                    <Header name={name} taken={names} onRename={(to) => onChange(renameInput(scene, grid, k, to))} />
                    {asking === k ? (
                      <span className="logic-ask">
                        {t('logic.table.removeAsk')}
                        <button onClick={() => askRemove(k)}>{t('logic.table.yes')}</button>
                        <button onClick={() => setAsking(null)}>{t('logic.table.no')}</button>
                      </span>
                    ) : (
                      <button
                        className="logic-head-remove"
                        disabled={line !== undefined}
                        title={line ? t('logic.table.inputUsed') : t('logic.table.removeInput')}
                        onClick={() => askRemove(k)}
                      >
                        ✕
                      </button>
                    )}
                  </th>
                );
              })}
              <th className="logic-add-head">
                <button className="logic-add" disabled={n >= MAX_INPUTS} title={t('logic.table.addInput')} onClick={() => onChange(addInput(scene, grid))}>
                  +
                </button>
              </th>
              {columns.map((column, c) => (
                <th key={`${column.index}-${column.name}`} className={c === 0 ? 'logic-output-head first' : 'logic-output-head'}>
                  <Header
                    name={column.name}
                    taken={names}
                    onRename={column.index === null ? undefined : (to) => onChange(renameOutputAt(scene, column.index!, column.name, to))}
                  />
                  {column.index !== null && (
                    <button className="logic-head-remove" title={t('logic.line.remove')} onClick={() => onChange(removeOutput(scene, column.index!))}>
                      ✕
                    </button>
                  )}
                </th>
              ))}
              <th className="logic-add-head">
                <button className="logic-add" title={t('logic.table.addOutput')} onClick={() => onChange(addOutput(scene, grid))}>
                  +
                </button>
              </th>
            </tr>
          </thead>
          <tbody>
            {Array.from({ length: rows }, (_, row) => (
              <tr key={row} className={row === currentRow ? 'current' : undefined}>
                <td className="logic-row-number">
                  {onRow ? (
                    <button className="logic-row-set" title={t('logic.sim.setRow')} onClick={() => onRow(row)}>
                      {row}
                    </button>
                  ) : (
                    row
                  )}
                </td>
                {inputs.map((name, k) => (
                  <td key={name} className="logic-bit">
                    {(row >> (n - 1 - k)) & 1}
                  </td>
                ))}
                <td className="logic-add-head" />
                {columns.map((column, c) => {
                  const value = column.values[row];
                  const wrong = column.index !== null && errorRows.get(column.index)?.has(row);
                  const classes = ['logic-cell', c === 0 ? 'first' : '', value === null ? 'empty' : '', wrong ? 'wrong' : ''].filter(Boolean).join(' ');
                  if (column.index === null)
                    return (
                      <td key={column.name} className={classes}>
                        {shown(value)}
                      </td>
                    );
                  const index = column.index;
                  return (
                    <td key={`${index}-${column.name}`} className={classes}>
                      <button
                        data-row={row}
                        data-col={c}
                        onClick={() => set(index, row, cycle(value))}
                        onKeyDown={(event) => onKey(event, index, row, c, value)}
                      >
                        {shown(value)}
                      </button>
                    </td>
                  );
                })}
                <td className="logic-add-head" />
              </tr>
            ))}
          </tbody>
          <tfoot>
            <tr>
              <td colSpan={n + 2} className="logic-table-copy">
                <CopyButton text={answer?.response.tableLatex ?? ''} hint={t('logic.copy.table')} />
              </td>
              {columns.map((column) => (
                <td key={`${column.index}-${column.name}`}>
                  {column.index !== null && column.source === 'table' && column.values.some((value) => value === null) && (
                    <button
                      className="logic-fill"
                      title={t('logic.table.fill.hint')}
                      onClick={() => onChange(editColumn(scene, grid, column.index!, (values) => values.map((v) => v ?? '0')))}
                    >
                      {t('logic.table.fill')}
                    </button>
                  )}
                </td>
              ))}
              <td />
            </tr>
          </tfoot>
        </table>
      </div>
    </section>
  );
}
