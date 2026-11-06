import { useRef, type CSSProperties, type KeyboardEvent } from 'react';
import type { UnknownNames } from '../api';
import { useT } from '../i18n';
import { defaultNames, MAX_GRID, type GridInput } from '../linear';
import { Katex } from './Katex';
import { MathField, type MathFieldHandle } from './MathField';

const clamp = (value: number) => Math.min(MAX_GRID, Math.max(1, Math.round(value)));

interface Props {
  grid: GridInput;
  names: UnknownNames;
  onChange(grid: GridInput): void;
  onSubmit(): void;
}

/**
 * `[A | b]` as a grid of small formula fields, the unknowns above their columns. Tab goes through
 * the cells row by row and does not leave the grid; Enter solves. An empty cell is 0.
 */
export function MatrixGrid({ grid, names, onChange, onSubmit }: Props) {
  const { t } = useT();
  const m = grid.m ?? grid.n;
  const headers = grid.unknowns?.length === grid.n ? grid.unknowns : defaultNames(names, grid.n);
  const cells = useRef(new Map<string, MathFieldHandle>());
  /** The right-hand side keeps its key when n changes, so a field never swaps between the two. */
  const key = (r: number, c: number) => (c === grid.n ? `${r}-rhs` : `${r}-${c}`);
  const focusCell = (r: number, c: number) => cells.current.get(key(r, c))?.focus();

  const setCell = (r: number, c: number, value: string) => {
    if (c === grid.n) {
      const rhs = [...grid.rhs];
      rhs[r] = value;
      onChange({ ...grid, rhs });
    } else {
      const coefficients = [...grid.coefficients];
      const row = [...(coefficients[r] ?? [])];
      row[c] = value;
      coefficients[r] = row;
      onChange({ ...grid, coefficients });
    }
  };
  const cell = (r: number, c: number) => (c === grid.n ? grid.rhs[r] : grid.coefficients[r]?.[c]) ?? '';

  const onKey = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'Enter') onSubmit();
  };

  return (
    <div className="matrix-input">
      <div className="task-inputs">
        <label>
          {t('linear.unknowns')}
          <input
            className="task-input matrix-size"
            type="number"
            min={1}
            max={MAX_GRID}
            value={grid.n}
            onChange={(event) => event.target.value && onChange({ ...grid, n: clamp(Number(event.target.value)) })}
            onKeyDown={onKey}
          />
        </label>
        <label>
          {t('linear.equations')}
          <input
            className="task-input matrix-size"
            type="number"
            min={1}
            max={MAX_GRID}
            value={grid.m ?? ''}
            placeholder={String(grid.n)}
            onChange={(event) => onChange({ ...grid, m: event.target.value ? clamp(Number(event.target.value)) : null })}
            onKeyDown={onKey}
          />
        </label>
      </div>

      <div className="matrix-grid" style={{ '--columns': grid.n } as CSSProperties}>
        {headers.map((name, c) => (
          <Katex key={`h${c}`} className="matrix-head" latex={name} />
        ))}
        <span className="matrix-head rhs" />
        {Array.from({ length: m }, (_, r) =>
          Array.from({ length: grid.n + 1 }, (_, c) => (
            <div className={c === grid.n ? 'matrix-cell rhs' : 'matrix-cell'} key={key(r, c)}>
              <MathField
                ref={(handle) => {
                  if (handle) cells.current.set(key(r, c), handle);
                  else cells.current.delete(key(r, c));
                }}
                className="cell-input"
                value={cell(r, c)}
                onChange={(value) => setCell(r, c, value)}
                onEnter={onSubmit}
                onTab={(direction) => {
                  // Row by row; the first and the last cell keep the focus.
                  const next = r * (grid.n + 1) + c + direction;
                  if (next >= 0 && next < m * (grid.n + 1)) focusCell(Math.floor(next / (grid.n + 1)), next % (grid.n + 1));
                }}
                autoFocus={r === 0 && c === 0}
              />
            </div>
          )),
        )}
      </div>
    </div>
  );
}
