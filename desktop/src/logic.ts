import type { LogicError, LogicOutput, LogicRequest, LogicResponse, LogicScene, LogicSource } from './api';

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
