import type { LinearSystemRequest, UnknownNames } from './api';
import type { TextKey } from './i18n/en';

/** The input of the linear systems mode, and how it becomes a request to the engine. */

export type LinearTab = 'equations' | 'matrix';

/** The rows of `[A | b]` with LaTeX entries, and the names of the columns if they were given. */
export interface MatrixInput {
  unknowns: string[] | null;
  rows: string[][];
}

/**
 * The grid of the matrix tab: n unknowns, m equations (null: as many as unknowns). Entries
 * outside n × m are kept, so making the grid smaller and larger again loses nothing.
 */
export interface GridInput {
  n: number;
  m: number | null;
  /** Coefficients by row and column; missing ones are empty, which the engine reads as 0. */
  coefficients: string[][];
  /** The right-hand side by row. */
  rhs: string[];
  /** Names from a loaded file, used while the number of unknowns matches them. */
  unknowns: string[] | null;
}

/** What the page holds; `App` keeps it, so it survives a switch to another mode and back. */
export interface LinearInput {
  tab: LinearTab;
  /** One equation per line, as typed. */
  lines: string[];
  grid: GridInput;
  /** A matrix loaded from JSON that is too large for the grid; it is solved instead of the grid. */
  large: MatrixInput | null;
}

export const MAX_GRID = 10;

export const EMPTY_LINEAR: LinearInput = {
  tab: 'equations',
  lines: [''],
  grid: { n: 3, m: null, coefficients: [], rhs: [], unknowns: null },
  large: null,
};

/** The rows of the grid as the engine takes them, every row n + 1 entries long. */
export function gridRows(grid: GridInput): string[][] {
  const m = grid.m ?? grid.n;
  return Array.from({ length: m }, (_, r) => [
    ...Array.from({ length: grid.n }, (_, c) => grid.coefficients[r]?.[c]?.trim() ?? ''),
    grid.rhs[r]?.trim() ?? '',
  ]);
}

/** The system as a request, or null while there is nothing to solve. */
export function requestFor(input: LinearInput): LinearSystemRequest | null {
  if (input.tab === 'equations') {
    const lines = input.lines.map((line) => line.trim()).filter(Boolean);
    return lines.length ? { equations: `\\begin{cases} ${lines.join(' \\\\ ')} \\end{cases}` } : null;
  }
  if (input.large) return matrixRequest(input.large.rows, input.large.unknowns);
  const rows = gridRows(input.grid);
  if (rows.every((row) => row.every((cell) => !cell))) return null;
  return matrixRequest(rows, input.grid.unknowns?.length === input.grid.n ? input.grid.unknowns : null);
}

/** Without names the engine names the columns by the setting. */
function matrixRequest(rows: string[][], unknowns: string[] | null): LinearSystemRequest {
  return { matrix: unknowns ? { unknowns, rows } : { rows } };
}

/** The letters of the setting `letters`, as in `quantus_core::eval::linear_system`: no `e`, no `i`. */
const LETTERS = 'xyzabcdfghjklmnopqrstuvw'.split('');

/** `x_{1} … x_{n}`, or `x, y, z, a, …, w` and then `x_{1}, x_{2}, …`, as the engine names them. */
export function defaultNames(names: UnknownNames, n: number): string[] {
  const indexed = (k: number) => `x_{${k}}`;
  if (names === 'indexed') return Array.from({ length: n }, (_, k) => indexed(k + 1));
  return Array.from({ length: n }, (_, k) => (k < LETTERS.length ? LETTERS[k] : indexed(k - LETTERS.length + 1)));
}

/** A broken JSON input: the text key and its values, translated where it is shown. */
export class MatrixJsonError extends Error {
  constructor(
    readonly key: TextKey,
    readonly values: Record<string, string | number> = {},
  ) {
    super(key);
  }
}

/**
 * Reads `{"unknowns": [...], "rows": [[...], ...]}` or just the rows: every entry a number or a
 * LaTeX string, every row as long as the first and at least a coefficient and the right-hand side.
 */
export function parseMatrixJson(text: string): MatrixInput {
  let data: unknown;
  try {
    data = JSON.parse(text);
  } catch {
    throw new MatrixJsonError('linear.json.invalid');
  }
  const object = typeof data === 'object' && data !== null && !Array.isArray(data) ? (data as Record<string, unknown>) : null;
  const rows = object ? object.rows : data;
  if (!Array.isArray(rows) || rows.length === 0 || !rows.every(Array.isArray)) throw new MatrixJsonError('linear.json.shape');
  const width = (rows[0] as unknown[]).length;
  if (width < 2) throw new MatrixJsonError('linear.json.short');
  const entries = (rows as unknown[][]).map((row, r) => {
    if (row.length !== width) throw new MatrixJsonError('linear.json.width', { row: r + 1, count: row.length, first: width });
    return row.map((entry, c) => {
      if (typeof entry === 'number' && Number.isFinite(entry)) return String(entry);
      if (typeof entry === 'string') return entry;
      throw new MatrixJsonError('linear.json.entry', { row: r + 1, column: c + 1 });
    });
  });
  const names = object?.unknowns;
  if (names === undefined || names === null) return { unknowns: null, rows: entries };
  if (!Array.isArray(names) || !names.every((name) => typeof name === 'string') || names.length !== width - 1) {
    throw new MatrixJsonError('linear.json.names', { count: width - 1 });
  }
  return { unknowns: names, rows: entries };
}

/** A loaded matrix fills the grid if it fits; a larger one is solved as it is, without the grid. */
export function withMatrix(input: LinearInput, matrix: MatrixInput): LinearInput {
  const n = matrix.rows[0].length - 1;
  const m = matrix.rows.length;
  if (n > MAX_GRID || m > MAX_GRID) return { ...input, large: matrix };
  const grid: GridInput = {
    n,
    m: m === n ? null : m,
    coefficients: matrix.rows.map((row) => row.slice(0, n)),
    rhs: matrix.rows.map((row) => row[n]),
    unknowns: matrix.unknowns,
  };
  return { ...input, grid, large: null };
}

/** `[A | b]` for the read-only preview of a large matrix. */
export function matrixLatex(matrix: MatrixInput): string {
  const n = matrix.rows[0].length - 1;
  const body = matrix.rows.map((row) => row.map((entry) => entry.trim() || '0').join(' & ')).join(' \\\\ ');
  return `\\left[\\begin{array}{${'c'.repeat(n)}|c} ${body} \\end{array}\\right]`;
}
