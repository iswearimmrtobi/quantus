import { invoke } from '@tauri-apps/api/core';

/** The engine contract (spec §5) as the renderer sees it. */

export type StepKind = 'symbolic' | 'substituted' | 'result' | 'algebraic' | 'note';

export interface Step {
  kind: StepKind;
  latex: string;
  rule?: string;
  explanation?: string;
  before?: string;
  after?: string;
}

export interface Variable {
  key: string;
  symbol: string;
  latex: string;
  value: string;
  unit: string;
}

export interface Calculation {
  input: string;
  latex: string;
  result: string;
  steps: Step[];
  alignBlock: string;
  value: string;
  /** Imaginary part of a complex result; `value` is then the real part. */
  imaginary?: string;
  unit: string;
  quantityName: string | null;
  dimension: string;
  variable: Variable | null;
  domainRestrictions?: string[];
  warning?: string;
}

export type ElementKind =
  | 'resistor'
  | 'wire'
  | 'voltageSource'
  | 'currentSource'
  | 'potentiometer'
  | 'powerLoad';

/**
 * One element of a circuit as `CircuitRequest` takes it. Parameters are the text the user typed
 * (`"2mm^2"`); blank or absent means missing. x, y and rotation belong to the editor and are
 * ignored by the solver.
 */
export interface CircuitElement {
  id: string;
  kind: ElementKind;
  a: number;
  b: number;
  wiper?: number;
  r?: string;
  material?: string;
  length?: string;
  area?: string;
  temp?: string;
  rOverride?: string;
  lPerM?: string;
  cPerM?: string;
  u?: string;
  internalResistance?: string;
  i?: string;
  internalConductance?: string;
  rTotal?: string;
  position?: string;
  p?: string;
  x?: number;
  y?: number;
  rotation?: number;
}

export type ReverseParameter =
  | 'resistance'
  | 'length'
  | 'area'
  | 'voltage'
  | 'current'
  | 'power'
  | 'totalResistance'
  | 'position';

export interface ReverseRequest {
  element: string;
  parameter: ReverseParameter;
  measure: {
    kind: 'nodeVoltage' | 'voltage' | 'current' | 'power' | 'relativeDrop';
    node?: number;
    element?: string;
    elements?: string[];
    source?: string;
  };
  value: string;
}

export interface CircuitRequest {
  nodes: number[];
  elements: CircuitElement[];
  reverse?: ReverseRequest | null;
  precision?: number | null;
}

/**
 * A computed value. `relative` is the value for a source of one unit, because the source has no
 * value; `text` is the German reading of a relative or unknown one ("Unbekannt: …").
 */
export interface CircuitValue {
  status: 'known' | 'relative' | 'unknown';
  result: string | null;
  value: string | null;
  unit: string | null;
  source: string | null;
  text: string | null;
}

export interface CircuitResponse {
  nodes: { id: number; voltage: CircuitValue }[];
  elements: {
    id: string;
    voltage: CircuitValue;
    current: CircuitValue;
    power: CircuitValue;
    /** Given, or U/I; `null` for a source. */
    resistance: CircuitValue | null;
    notes: string[];
  }[];
  /** The parameter found; the values above are those with it. */
  reverse: { element: string; parameter: ReverseParameter; value: CircuitValue } | null;
}

export type BinomialForm = 'auto' | 'expanded' | 'binomial';
export type Mode = 'algebra' | 'physics' | 'imaginary' | 'electronics' | 'analysis' | 'linear';
export type QuadraticFormula = 'auto' | 'pq' | 'abc';
export type Language = 'en' | 'de';
export type DenominatorForm = 'auto' | 'expanded' | 'factored';
export type ConditionalIdentities = 'withCondition' | 'alwaysValid';
export type StepDetail = 'none' | 'brief' | 'standard' | 'detailed' | 'maximal';
export type Elimination = 'gauss' | 'gaussJordan';
export type UnknownNames = 'indexed' | 'letters';

/** What `quantus_core::settings::Settings` serialises to. */
export interface Settings {
  mode: Mode;
  precision: number;
  binomialForm: BinomialForm;
  completeSquare: boolean;
  denominatorForm: DenominatorForm;
  /** Whether an identity like √a·√b = √(ab) is applied with its condition (a, b ≥ 0) or only where it holds anyway. */
  conditionalIdentities: ConditionalIdentities;
  exactRoots: boolean;
  aAndYAsVariables: boolean;
  letterProducts: boolean;
  extraUnitLetters: boolean;
  language: Language;
  /** How the analysis mode solves quadratics; `auto` is pq in German and abc in English. */
  quadraticFormula: QuadraticFormula;
  /** How much working the step list shows; `none` hides it. */
  stepDetail: StepDetail;
  /** Whether a cancelled factor is struck through rather than coloured. */
  cancelMarks: boolean;
  /** How the steps reduce a system of linear equations: school Gauss or Gauss–Jordan. */
  elimination: Elimination;
  /** How the linear systems mode names the unknowns of a matrix: `x_1 … x_n` or `x, y, z, a, …`. */
  unknownNames: UnknownNames;
  /** Whether `i` is the imaginary unit in the linear systems mode rather than an unknown. */
  linearComplex: boolean;
}

/** A linear system as typed (`2x + y = 5; x - y = 1` or a `cases` block) or as the rows of `[A | b]`. */
export type LinearSystemRequest =
  | { equations: string }
  /** Entries in LaTeX, an empty one is 0; without `unknowns` the columns are named by `unknownNames`. */
  | { matrix: { unknowns?: string[]; rows: string[][] } };

/** What `quantus_core::api::LinearSystemResponse` serialises to. */
export interface LinearSystem {
  /** The unknowns in LaTeX, in the order of the columns: `x_{1}`. */
  unknowns: string[];
  /** `[A | b]` as LaTeX entries, one row per equation. */
  matrix: string[][];
  /** Each row as an equation in the unknowns, for the switch to the equations tab. */
  equations: string[];
  steps: Step[];
  /** Exact: `x_{1} = \frac{1}{2} \land x_{2} = 4`. */
  result: string;
  alignBlock: string;
}

/** A segment of the graph: points in order, broken at poles and gaps. */
export type Segment = [number, number][];

export interface AnalysisSection {
  id:
    | 'derivatives'
    | 'domain'
    | 'symmetry'
    | 'yIntercept'
    | 'zeros'
    | 'extrema'
    | 'inflection'
    | 'monotonicity'
    | 'curvature'
    | 'behaviour'
    | 'antiderivative';
  title: string;
  status: 'ok' | 'partial' | 'failed';
  message: string | null;
  steps: Step[];
  result: string | null;
}

export type PointKind =
  | 'zero'
  | 'yIntercept'
  | 'max'
  | 'min'
  | 'inflection'
  | 'saddle'
  | 'hole'
  | 'boundaryMax'
  | 'boundaryMin'
  | 'tangent';

export interface MarkedPoint {
  kind: PointKind;
  name: string;
  x: number;
  y: number;
  latex: string;
  /** For a member of a periodic family: the distance to the next member. */
  period: number | null;
}

export interface Asymptote {
  kind: 'vertical' | 'horizontal' | 'oblique';
  side: 'plusInfinity' | 'minusInfinity' | 'both' | null;
  x: number | null;
  slope: number | null;
  intercept: number | null;
  latex: string;
  /** For the poles of a periodic family (tan): the distance to the next one. */
  period: number | null;
}

export interface PlotData {
  xMin: number;
  xMax: number;
  yMin: number;
  yMax: number;
  segments: Segment[];
}

/** What `quantus_core::api::AnalysisResponse` serialises to. */
export interface Analysis {
  function: string;
  variable: string;
  sections: AnalysisSection[];
  points: MarkedPoint[];
  asymptotes: Asymptote[];
  period: string | null;
  plot: PlotData;
}

export interface Line {
  kind: 'line' | 'vertical';
  slope: number | null;
  intercept: number | null;
  x: number | null;
  latex: string;
}

export interface TangentResult {
  steps: Step[];
  point: MarkedPoint;
  tangent: Line;
  normal: Line;
}

export interface AreaResult {
  steps: Step[];
  area: string;
  signed: string;
  regions: { from: number; to: number; positive: boolean; segments: Segment[] }[];
}

export interface TableResult {
  /** `y` and `yValue` are null outside the domain. */
  rows: { x: string; y: string | null; xValue: number; yValue: number | null }[];
}

export interface EngineInfo {
  application: string;
  precision: number;
}

/** An error the backend explained in words — [position] points into the input (F2). */
export class ApiError extends Error {
  constructor(message: string, readonly position: number | null = null) {
    super(message);
    this.name = 'ApiError';
  }
}

/** One session per running app; the engine keeps its variables for as long as the app lives. */
const SESSION_ID = 'desktop';

/** What `quantus_core::api::ApiError` serialises to. */
interface ErrorPayload {
  error?: string;
  position?: number | null;
}

async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (failure) {
    if (typeof failure === 'object' && failure !== null && 'error' in failure) {
      const { error, position } = failure as ErrorPayload;
      throw new ApiError(error ?? 'Unknown error.', position ?? null);
    }
    throw new ApiError(String(failure));
  }
}

export const api = {
  info: () => call<EngineInfo>('engine_info'),

  evaluate: (latex: string) => call<Calculation>('evaluate', { request: { latex, sessionId: SESSION_ID } }),

  solveCircuit: (request: CircuitRequest) => call<CircuitResponse>('solve_circuit', { request }),

  /** Reads a circuit file; a broken one is an `ApiError` and nothing is returned. */
  loadCircuit: (path: string) => call<unknown>('load_circuit', { path }),

  saveCircuit: (path: string, circuit: unknown) => call<void>('save_circuit', { path, circuit }),

  /** The curve sketch; the variable is read off the input (`f(t) = …`). */
  analyze: (latex: string) => call<Analysis>('analyze', { request: { latex } }),

  /** Fresh curve segments for a panned or zoomed view; the y-range is for clipping. */
  sampleFunction: (latex: string, view: { xMin: number; xMax: number; yMin: number; yMax: number }, samples: number) =>
    call<{ segments: Segment[] }>('sample_function', { request: { latex, ...view, samples } }),

  tangent: (latex: string, x0: string) => call<TangentResult>('tangent', { request: { latex, x0 } }),

  /** Without bounds, the area between the outermost zeros. */
  areaUnderCurve: (latex: string, from: string | null, to: string | null) =>
    call<AreaResult>('area_under_curve', { request: { latex, from, to } }),

  valueTable: (latex: string, from: string, to: string, step: string) =>
    call<TableResult>('value_table', { request: { latex, from, to, step } }),

  /** Solves a linear system; it uses no session variables. */
  solveLinearSystem: (request: LinearSystemRequest) => call<LinearSystem>('solve_linear_system', { request }),

  variables: () => call<Variable[]>('list_variables', { sessionId: SESSION_ID }),

  forget: (key: string) => call<void>('delete_variable', { key, sessionId: SESSION_ID }),

  settings: () => call<Settings>('get_settings'),

  /** Checks and saves; returns what the engine now uses. */
  saveSettings: (settings: Settings) => call<Settings>('set_settings', { settings }),
};
