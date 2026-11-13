import type { LogicError, LogicOutput, LogicRequest, LogicResponse, LogicScene, LogicSource, LogicValue } from './api';

/**
 * The Logic mode's scene as the page keeps it (design/digital-logic.md §5.1): `App` holds it, so
 * that it survives a switch to another mode and back, and the session saves it.
 */

/** A new scene: one empty function line. */
export const EMPTY_LOGIC: LogicScene = { inputs: [], outputs: [{ source: { kind: 'expression', latex: '' } }] };

/** An output given by a typed line. */
export type LineOutput = LogicOutput & { source: Extract<LogicSource, { latex: string }> };

/** Whether an output is given by a typed line, an expression or a minterm list. */
export const isLine = (output: LogicOutput): output is LineOutput => output.source.kind === 'expression' || output.source.kind === 'minterms';

/** A new, empty function line. */
export const newLine = (): LogicOutput => ({ source: { kind: 'expression', latex: '' } });

/** Whether a line is still empty; such lines are not sent. */
const blank = (output: LogicOutput) => isLine(output) && !output.source.latex.trim();

/** The request for a scene, and for each of its outputs the index in the scene they came from. */
export interface SentScene {
  request: LogicRequest | null;
  /** `indices[k]` is the scene's output that the request's (and the response's) output `k` is. */
  indices: number[];
}

/** The scene as the engine takes it: empty lines left out; null when nothing is left to work out. */
export function logicRequest(scene: LogicScene): SentScene {
  const indices = scene.outputs.flatMap((output, index) => (blank(output) ? [] : [index]));
  if (!indices.length && !scene.circuit) return { request: null, indices };
  return { request: { scene: { ...scene, outputs: indices.map((index) => scene.outputs[index]) } }, indices };
}

/** The answer with the outputs and errors given by the scene's indices. */
export interface LogicAnswer {
  response: LogicResponse;
  /** The response's output for each of the scene's outputs; null for an empty line. */
  outputs: (LogicResponse['outputs'][number] | null)[];
  /** The response's outputs that the scene has no line or column for: lamps of the circuit. */
  extra: LogicResponse['outputs'];
  /** The errors, `output` the scene's index. */
  errors: LogicError[];
}

export function logicAnswer(scene: LogicScene, sent: SentScene, response: LogicResponse): LogicAnswer {
  const outputs = scene.outputs.map((_, index) => {
    const k = sent.indices.indexOf(index);
    return k < 0 ? null : (response.outputs[k] ?? null);
  });
  const errors = response.errors.map((error) => ({ ...error, output: error.output === null ? null : (sent.indices[error.output] ?? error.output) }));
  return { response, outputs, extra: response.outputs.slice(sent.indices.length), errors };
}

/** The kind of a typed line: a list of minterms or maxterms, or an expression. */
export function lineKind(latex: string): 'expression' | 'minterms' {
  // The body after an optional `f =` or `f(A, B) =`.
  const body = latex.replace(/^[^=]*=/, '').trim();
  return /^(\\sum|\\Sigma|\\prod|\\Pi|[mM]\s*(\(|\\left\s*\()\s*\d)/.test(body) ? 'minterms' : 'expression';
}

/** The source of a typed line. */
export const lineSource = (latex: string): LogicSource => ({ kind: lineKind(latex), latex });

/** The scene a session saved, or a new one. */
export function logicInput(request: LogicRequest | null | undefined): LogicScene {
  const scene = request?.scene;
  if (!scene || (!scene.outputs?.length && !scene.circuit)) return EMPTY_LOGIC;
  return { ...scene, inputs: scene.inputs ?? [], outputs: scene.outputs ?? [] };
}

/** What the session keeps: the scene, or nothing while it is new. */
export function logicSaved(scene: LogicScene): LogicRequest | null {
  return JSON.stringify(scene) === JSON.stringify(EMPTY_LOGIC) ? null : { scene };
}

// ---- The truth table (§5.4).

/** At most this many inputs (256 rows). */
export const MAX_INPUTS = 8;

/** A row's value, or null for a cell not filled. */
export type Cell = LogicValue | null;

/** Bit `k` of `row` over `n` inputs; the first input is the most significant bit. */
const bit = (row: number, k: number, n: number) => (row >> (n - 1 - k)) & 1;

/**
 * A column over the inputs `from` written over the inputs `to`, which hold all of them: each row
 * of `to` takes the value of the row of `from` with the same bits. Null when an input is missing.
 */
export function remap(values: Cell[], from: string[], to: string[]): Cell[] | null {
  const at = from.map((name) => to.indexOf(name));
  if (at.some((k) => k < 0) || values.length !== 1 << from.length) return null;
  const n = to.length;
  return Array.from({ length: 1 << n }, (_, row) => values[at.reduce((old, k, j) => old | (bit(row, k, n) << (from.length - 1 - j)), 0)]);
}

/** Whether a column changes with input `k`: some two rows differing only in it have other values. */
export function dependsOn(values: Cell[], n: number, k: number): boolean {
  const step = 1 << (n - 1 - k);
  return values.some((value, row) => (row & step) === 0 && value !== values[row | step]);
}

/** A column with input `k` taken out: the rows where it is 0. */
export function withoutInput(values: Cell[], n: number, k: number): Cell[] {
  return values.filter((_, row) => bit(row, k, n) === 0);
}

/** `A`, `x_{1}`, `\mathrm{Start}`, as the core writes a name. */
export function nameLatex(name: string): string {
  const [base, index] = name.split(/_(.*)/s);
  if (index !== undefined) return `${base}_{${index}}`;
  return name.length === 1 ? name : `\\mathrm{${name}}`;
}

/** The words the parser reads as operators, never as names. */
const WORDS = ['and', 'or', 'not', 'xor', 'xnor', 'nand', 'nor'];

/**
 * A name as typed in a header, `x_{12}` as `x_12`, or null when it is no name: a letter, then
 * letters and digits, or a letter with an index of letters and digits; no operator word.
 */
export function readName(text: string): string | null {
  const name = text.replace(/[{}\s]/g, '');
  if (!/^[A-Za-z]([A-Za-z0-9]*|_[A-Za-z0-9]+)$/.test(name) || WORDS.includes(name.toLowerCase())) return null;
  return name;
}

/** The commands whose braced argument is a name, `\mathrm{Start}`. */
const NAME_COMMANDS = ['mathrm', 'text', 'operatorname', 'mathit'];

/**
 * A line with the variable `from` renamed to `to` wherever the parser would read it: a letter
 * (with its index) in a run of letters, or a name in `\mathrm{…}`; operator words and other
 * commands are left alone.
 */
export function renameVariable(latex: string, from: string, to: string): string {
  const replacement = nameLatex(to);
  let out = '';
  let i = 0;
  while (i < latex.length) {
    const c = latex[i];
    if (c === '\\') {
      const command = /^\\([A-Za-z]+)/.exec(latex.slice(i));
      if (!command) {
        out += latex.slice(i, i + 2);
        i += 2;
        continue;
      }
      i += command[0].length;
      const argument = /^\s*\{([^{}]*)\}/.exec(latex.slice(i));
      if (NAME_COMMANDS.includes(command[1]) && argument) {
        i += argument[0].length;
        const name = argument[1].replace(/\s/g, '');
        out += name === from ? replacement : `${command[0]}${argument[0]}`;
      } else out += command[0];
      continue;
    }
    if (/[A-Za-z]/.test(c)) {
      const run = /^[A-Za-z]+/.exec(latex.slice(i))![0];
      i += run.length;
      if (WORDS.includes(run.toLowerCase())) {
        out += run;
        continue;
      }
      // The last letter of the run takes the index, `AB_1` is A and B_1.
      const index = /^_\s*(\{[^{}]*\}|[A-Za-z0-9])/.exec(latex.slice(i));
      if (index) i += index[0].length;
      for (let k = 0; k < run.length; k++) {
        const last = k === run.length - 1;
        const name = last && index ? `${run[k]}_${index[1].replace(/[{}\s]/g, '')}` : run[k];
        out += name === from ? replacement : last && index ? `${run[k]}${index[0]}` : run[k];
      }
      continue;
    }
    out += c;
    i += 1;
  }
  return out;
}

/** A line with its output renamed, when it names it (`f = …`, `f(A, B) = …`). */
export function renameOutput(latex: string, from: string, to: string): string {
  const left = /^[^=(]*(?=\(|\\left|=)/.exec(latex);
  if (!left || !latex.includes('=')) return latex;
  return renameVariable(left[0], from, to) + latex.slice(left[0].length);
}

/** The first of `candidates` not among `taken`, then `base_1`, `base_2`, … */
export function freeName(candidates: string[], taken: string[], base: string): string {
  const free = candidates.find((name) => !taken.includes(name));
  if (free) return free;
  for (let k = 1; ; k++) if (!taken.includes(`${base}_${k}`)) return `${base}_${k}`;
}

/** An output column of the table. */
export interface Column {
  /** The scene's output; null for a lamp of the circuit that no output names. */
  index: number | null;
  name: string;
  source: LogicSource['kind'];
  values: Cell[];
}

/** The table as shown: the inputs, and a column per output that is not an empty line. */
export interface Grid {
  inputs: string[];
  columns: Column[];
}

const empty = (n: number): Cell[] => Array.from({ length: 1 << n }, () => null);

/**
 * The table of the scene: its columns over the inputs the core found (the scene's own while it
 * has none); a table's column is the scene's, the others are the core's.
 */
export function logicGrid(scene: LogicScene, answer: LogicAnswer | null): Grid {
  const inputs = answer?.response.inputs.length ? answer.response.inputs : scene.inputs;
  const rows = 1 << inputs.length;
  const columns: Column[] = [];
  scene.outputs.forEach((output, index) => {
    if (blank(output)) return;
    const result = answer?.outputs[index] ?? null;
    const values =
      output.source.kind === 'table'
        ? (remap(output.source.values, scene.inputs, inputs) ?? (output.source.values.length === rows ? output.source.values : empty(inputs.length)))
        : result?.table.length === rows
          ? result.table
          : empty(inputs.length);
    columns.push({ index, name: result?.name ?? output.name ?? '', source: output.source.kind, values });
  });
  for (const lamp of answer?.extra ?? []) columns.push({ index: null, name: lamp.name, source: lamp.source, values: lamp.table.length === rows ? lamp.table : empty(inputs.length) });
  return { inputs, columns };
}

/** The scene with `inputs` as its table's columns, each table rewritten over them. */
function overInputs(scene: LogicScene, grid: Grid, inputs: string[], column: (values: Cell[]) => Cell[]): LogicScene {
  const outputs = scene.outputs.map((output, index) => {
    if (output.source.kind !== 'table') return output;
    const shown = grid.columns.find((c) => c.index === index)?.values ?? output.source.values;
    return { ...output, source: { kind: 'table' as const, values: column(shown) } };
  });
  return { ...scene, inputs, outputs };
}

/**
 * The scene with output `index` taken from the table, its column changed by `change`: an output
 * from a line or the circuit becomes a table of the values it had (§5.1).
 */
export function editColumn(scene: LogicScene, grid: Grid, index: number, change: (values: Cell[]) => Cell[]): LogicScene {
  const next = overInputs(scene, grid, grid.inputs, (values) => values);
  const column = grid.columns.find((c) => c.index === index);
  if (!column) return scene;
  const outputs = next.outputs.map((output, i) =>
    i === index ? { ...output, name: column.name, source: { kind: 'table' as const, values: change(column.values) } } : output,
  );
  return { ...next, outputs };
}

/** The scene with a new input at the end: every table keeps its values for both of its rows. */
export function addInput(scene: LogicScene, grid: Grid): LogicScene {
  const taken = [...grid.inputs, ...grid.columns.map((c) => c.name)];
  const name = freeName('ABCDEFGHIJKLMNOPQRSTUVWXYZ'.split(''), taken, 'x');
  const inputs = [...grid.inputs, name];
  return overInputs(scene, grid, inputs, (values) => remap(values, grid.inputs, inputs) ?? empty(inputs.length));
}

/** The scene without input `k`: every table keeps its rows where the input is 0. */
export function removeInput(scene: LogicScene, grid: Grid, k: number): LogicScene {
  const n = grid.inputs.length;
  return overInputs(scene, grid, grid.inputs.filter((_, i) => i !== k), (values) => withoutInput(values, n, k));
}

/** Whether a line names the variable `name`. */
export const mentions = (latex: string, name: string) => renameVariable(latex, name, '#') !== latex;

/** The scene with input `k` renamed, in the table and in every line. */
export function renameInput(scene: LogicScene, grid: Grid, k: number, to: string): LogicScene {
  const from = grid.inputs[k];
  const next = overInputs(scene, grid, grid.inputs.map((name, i) => (i === k ? to : name)), (values) => values);
  return { ...next, outputs: next.outputs.map((output) => (isLine(output) ? { ...output, source: lineSource(renameVariable(output.source.latex, from, to)) } : output)) };
}

/** The scene with output `index` renamed, in its line too when that names it. */
export function renameOutputAt(scene: LogicScene, index: number, from: string, to: string): LogicScene {
  return {
    ...scene,
    outputs: scene.outputs.map((output, i) =>
      i !== index ? output : isLine(output) ? { ...output, name: to, source: lineSource(renameOutput(output.source.latex, from, to)) } : { ...output, name: to },
    ),
  };
}

/** The scene with a new, empty table column; with no inputs yet, over A and B. Empty lines go. */
export function addOutput(scene: LogicScene, grid: Grid): LogicScene {
  const inputs = grid.inputs.length ? grid.inputs : ['A', 'B'];
  const next = overInputs(scene, grid, inputs, (values) => values);
  const taken = [...inputs, ...grid.columns.map((c) => c.name)];
  const name = freeName('fghpqrsuvwyz'.split(''), taken, 'f');
  return { ...next, outputs: [...next.outputs.filter((output) => !blank(output)), { name, source: { kind: 'table', values: empty(inputs.length) } }] };
}

/** The scene without output `index`; the last one leaves an empty line. */
export function removeOutput(scene: LogicScene, index: number): LogicScene {
  const outputs = scene.outputs.filter((_, i) => i !== index);
  return { ...scene, outputs: outputs.length ? outputs : [newLine()] };
}

/** The next value of a cell clicked: 0 → 1 → – → 0; an empty one becomes 0. */
export const cycle = (value: Cell): LogicValue => (value === '0' ? '1' : value === '1' ? '-' : '0');
