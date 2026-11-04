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
  /** The unit of the result in LaTeX, empty for a plain number; what ENG keeps after the value. */
  unitLatex: string;
  /** The other ways to write the result, for the form switch; absent when there are none. */
  forms?: ResultForm[];
}

export type FormKind =
  | 'fraction'
  | 'mixedFraction'
  | 'recurring'
  | 'decimal'
  | 'exact'
  | 'primeFactors'
  | 'sexagesimal'
  | 'rectangular'
  | 'polar'
  | 'dec'
  | 'hex'
  | 'bin'
  | 'oct';

/** What `quantus_core::forms::ResultForm` serialises to. */
export interface ResultForm {
  kind: FormKind;
  latex: string;
}

/** A letter of an input that has no value yet (CALC, SOLVE). */
export interface FreeVariable {
  key: string;
  symbol: string;
}

/** A value for a letter, in LaTeX. */
export interface Assignment {
  key: string;
  latex: string;
}

export interface ConstantInfo {
  key: string;
  /** What the picker inserts: `c_{0}`. */
  latex: string;
  name: string;
  /** The value with its unit in LaTeX, to 12 digits. */
  value: string;
}

export interface ConstantCategory {
  category: string;
  name: string;
  constants: ConstantInfo[];
}

/** Units a conversion can aim at, by category. */
export interface UnitCategory {
  name: string;
  units: UnitInfo[];
}

/** A target unit: the LaTeX after `\blacktriangleright` and its name in the set language. */
export interface UnitInfo {
  latex: string;
  name: string;
}

/** A hit of the quick search: a unit goes in after `\blacktriangleright`, a constant as it is. */
export interface LibraryHit {
  kind: 'constant' | 'unit';
  latex: string;
  name: string;
  /** The value with unit for a constant, the category for a unit. */
  detail: string;
}

/** A session as the session menu lists it; times are UTC milliseconds. */
export interface SessionInfo {
  id: string;
  name: string;
  created: number | null;
  lastUsed: number | null;
  /** How many calculations its history holds. */
  entries: number;
}

/** What a session keeps of the modes with their own input; the desktop fills `linear`. */
export interface ModeInputs {
  linear?: LinearSystemRequest | null;
  geometry?: GeometryRequest | null;
  [mode: string]: unknown;
}

/** One calculation of a session's history, oldest first. */
export interface HistoryEntry {
  input: string;
  response: Calculation;
  at?: number;
}

/** One line of a solution block: an optional sentence and an equation. */
export interface WriteupLine {
  kind: 'law' | 'rearranged' | 'substituted' | 'result' | 'unitCheck';
  text?: string;
  latex: string;
}

export interface WriteupBlock {
  target: string;
  /** A later calculation uses it: a step on the way, not wanted. */
  intermediate: boolean;
  lines: WriteupLine[];
}

/** The session written up as given, wanted and solution (Gegeben / Gesucht / Lösung). */
export interface Writeup {
  given: string[];
  wanted: string[];
  blocks: WriteupBlock[];
  /** All of it as LaTeX for amsmath, to copy. */
  latex: string;
  /** All of it as Markdown with `$…$` math, to copy. */
  markdown: string;
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

/** A grid point, in grid steps. */
export interface GridPoint {
  x: number;
  y: number;
}

/**
 * A part as the Kirchhoff mode takes it: the parameters of a `CircuitElement`, and instead of
 * node numbers the grid points of its terminals. The engine works the nodes out itself.
 */
export interface KirchhoffElement extends Omit<CircuitElement, 'kind' | 'a' | 'b'> {
  kind: ElementKind | 'ground';
  pa: GridPoint;
  pb: GridPoint;
  /** Flips the reference direction of the branch this part leads. */
  reversed?: boolean;
}

export interface KirchhoffRequest {
  elements: KirchhoffElement[];
  precision?: number | null;
  /** The grid points of A and B: then the answer is the resistance between them. */
  terminals?: [GridPoint, GridPoint] | null;
}

/** A part in a branch or loop; `forward` if it is passed from `a` to `b`. */
export interface KirchhoffMember {
  id: string;
  /** A source's internal resistance, which has no part of its own on the canvas. */
  internal: boolean;
  forward: boolean;
}

/** What `quantus_core::api::KirchhoffResponse` serialises to. */
export interface Kirchhoff {
  junctions: { name: string; anchor: GridPoint; symbolic: string; numeric: string; dropped: boolean }[];
  /** Clockwise; the anchor lies inside the loop, in grid steps (not necessarily whole). */
  loops: { name: string; anchor: GridPoint; members: KirchhoffMember[]; symbolic: string; numeric: string }[];
  /** `current` is `I_{1}`, or a current source's, signed: `-I_{q,2}`. */
  branches: { current: string; members: KirchhoffMember[]; lead: string }[];
  unknowns: string[];
  matrix: string[][];
  steps: Step[];
  result: string[];
  /** U, I and P per part, empty without a solution. */
  elements: CircuitResponse['elements'];
  powerBalance: { generated: CircuitValue; consumed: CircuitValue; latex: string } | null;
  notes: string[];
  alignBlock: string;
  /** The equations have no single solution; the steps end where that shows. */
  error: string | null;
  /** The resistance between A and B; each merge names the parts it stands for, in step order. */
  equivalent: { latex: string; resistance: CircuitValue; merges: { symbol: string; parts: string[]; series: boolean }[] } | null;
}

/**
 * A point or vector in a form: its coordinate cells (two or three, an empty one is 0), or one
 * LaTeX entry such as `A`, `(1|2|3)`, `\vec{a}` or `B - A`.
 */
export type GeometryEntry = string[] | string;

/** How an object of the Geometry scene is given; numbers are LaTeX, `sphere`/`plane` of a cut are names. */
export type GeometryShape =
  | { kind: 'point'; at: GeometryEntry }
  | { kind: 'vector'; value: GeometryEntry }
  | { kind: 'line'; support: GeometryEntry; direction: GeometryEntry }
  | { kind: 'lineThrough'; a: GeometryEntry; b: GeometryEntry }
  | { kind: 'plane'; support: GeometryEntry; u: GeometryEntry; v: GeometryEntry }
  | { kind: 'planeNormal'; support: GeometryEntry; normal: GeometryEntry }
  | { kind: 'planeThrough'; a: GeometryEntry; b: GeometryEntry; c: GeometryEntry }
  | { kind: 'planeCoordinate'; a: string; b: string; c: string; d: string }
  | { kind: 'sphere'; centre: GeometryEntry; radius: string }
  | { kind: 'sphereThrough'; centre: GeometryEntry; point: GeometryEntry }
  | { kind: 'sphereFour'; a: GeometryEntry; b: GeometryEntry; c: GeometryEntry; d: GeometryEntry }
  | { kind: 'circle'; centre: GeometryEntry; radius: string; normal?: GeometryEntry | null }
  | { kind: 'circleCut'; sphere: string; plane: string }
  /** One typed line with its name: `A(1|2|3)`, `g: X = A + \lambda \vec{u}`, `E: 2x + y = 3`. */
  | { kind: 'typed'; latex: string };

/** A named object; a typed one takes its name from the line. */
export type GeometryObject = GeometryShape & { name?: string };

export type GeometryMeasure = 'length' | 'surface' | 'volume' | 'circumference' | 'area';
export type PlaneForm = 'parameter' | 'normal' | 'coordinate';

/** What is asked about the scene; objects by name, a vector also as `\vec{a}`. */
export type GeometryTask =
  | { kind: 'relation' | 'distance' | 'angle' | 'midpoint'; a: string; b: string }
  | { kind: 'convert'; plane: string; form: PlaneForm }
  | { kind: 'foot'; point: string; onto: string }
  | { kind: 'reflect'; point: string; mirror: string }
  | { kind: 'tangent'; object: string; point: string }
  | { kind: 'measure'; object: string; measure: GeometryMeasure }
  | { kind: 'evaluate'; latex: string };

export interface GeometryRequest {
  scene: { objects: GeometryObject[] };
  task?: GeometryTask | null;
}

export type GeometryKind = 'point' | 'vector' | 'line' | 'plane' | 'sphere' | 'circle';

/** What `quantus_core::api::GeometryResponse` serialises to. */
export interface Geometry {
  /** In scene order; `kind` is `typed` for a typed line that could not be read. */
  objects: { name: string; kind: GeometryKind | 'typed'; latex: string; steps: Step[]; error: string | null }[];
  task: {
    /** The result lines: the point, the equation, the relation, the angle. */
    result: string[];
    /** For a relation: `intersecting`, `skew`, `parallel`, `tangent`, `inside`, `on`, … */
    relation: string | null;
    steps: Step[];
    alignBlock: string;
    error: string | null;
  } | null;
}

/** A request for the 3D view: the camera's angles in radians, and a zoom on top of the fit. */
export interface GeometryViewRequest {
  scene: { objects: GeometryObject[] };
  yaw: number;
  pitch: number;
  zoom: number;
  hatch?: boolean;
}

/** A point on the screen in view units, y up; the fitted box lies inside the unit circle. */
export type ViewPoint = [number, number];
export type ViewStroke = 'axis' | 'tick' | 'object' | 'hatch' | 'hidden';

/** What `quantus_core::geometry::view::Primitive` serialises to; `object` indexes `objects`. */
export type ViewPrimitive =
  | { kind: 'segment'; from: ViewPoint; to: ViewPoint; depth: number; stroke: ViewStroke; object: number | null }
  | { kind: 'polygon'; points: ViewPoint[]; depth: number; object: number }
  | { kind: 'marker'; at: ViewPoint; depth: number; object: number }
  | { kind: 'label'; at: ViewPoint; depth: number; text: string; object: number | null };

/** The scene drawn: far to near, then the labels. */
export interface GeometryViewResponse {
  primitives: ViewPrimitive[];
  /** The objects that could be drawn, in scene order; `name` as keyed (`P_1`, `a` for `\vec{a}`). */
  objects: { name: string; kind: GeometryKind }[];
  camera: { yaw: number; pitch: number; zoom: number; centre: [number, number, number] };
  bounds: { min: [number, number, number]; max: [number, number, number] };
}

export type BinomialForm = 'auto' | 'expanded' | 'binomial';
export type Mode = 'algebra' | 'physics' | 'imaginary' | 'electronics' | 'analysis' | 'linear' | 'kirchhoff' | 'baseN' | 'verify' | 'geometry';
export type QuadraticFormula = 'auto' | 'pq' | 'abc';
export type Language = 'en' | 'de';
export type DenominatorForm = 'auto' | 'expanded' | 'factored';
export type ConditionalIdentities = 'withCondition' | 'alwaysValid';
export type StepDetail = 'none' | 'brief' | 'standard' | 'detailed' | 'maximal';
export type Elimination = 'gauss' | 'gaussJordan';
export type UnknownNames = 'indexed' | 'letters';
export type AngleUnit = 'degree' | 'radian';
export type NumberFormat =
  | { style: 'auto' | 'norm1' | 'norm2' }
  | { style: 'fix' | 'sci'; digits: number };
export type FractionForm = 'improper' | 'mixed';
export type ComplexForm = 'rectangular' | 'polar';
export type NumberBase = 'dec' | 'hex' | 'bin' | 'oct';

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
  /** What sin, cos and tan take and their inverses give; ° and rad on a value hold in either. */
  angleUnit: AngleUnit;
  /** How numbers are written: as before (`auto`), Norm 1/2, Fix or Sci with 0–9 digits. */
  numberFormat: NumberFormat;
  /** Plain numbers with k, M, m, µ …: `4{,}7\,\mathrm{k}`. */
  decimalPrefixes: boolean;
  /** Thousands separated by a thin space. */
  digitGrouping: boolean;
  /** Which fraction comes first: 7/3 or 2 1/3. */
  fractionForm: FractionForm;
  /** Whether a recurring decimal is among a result's forms. */
  recurring: boolean;
  /** Complex results as a + bi or r∠θ. */
  complexForm: ComplexForm;
  /** The base the Basis-N mode reads and writes. */
  numberBase: NumberBase;
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

/** The open session; every call that reads or changes session state goes to it. */
let sessionId = '';

export function setSessionId(id: string): void {
  sessionId = id;
}

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

  evaluate: (latex: string) => call<Calculation>('evaluate', { request: { latex, sessionId, at: Date.now() } }),

  /** One ENG (steps ≥ 0) or ←ENG (steps < 0) step of a result; 0 is the first press. */
  engineering: (calculation: Calculation, steps: number) =>
    call<{ latex: string }>('engineering', {
      request: { value: calculation.value, imaginary: calculation.imaginary ?? null, unitLatex: calculation.unitLatex, steps },
    }),

  /** The letters of an input that have no value in the session. */
  freeVariables: (latex: string) => call<FreeVariable[]>('free_variables', { request: { latex, sessionId } }),

  /** CALC: the values become session variables, then the input is evaluated. */
  calc: (latex: string, values: Assignment[]) => call<Calculation>('calc', { request: { latex, values, sessionId } }),

  /** SOLVE the input for one letter; the others from `values` or the session. */
  solveFor: (latex: string, variable: string, values: Assignment[], start: string | null) =>
    call<Calculation>('solve_for', { request: { latex, variable, values, start, sessionId } }),

  /** The physical constants by category, in the language of the settings. */
  constants: () => call<ConstantCategory[]>('constants'),

  /** The units `\blacktriangleright` can convert to, by category. */
  conversionUnits: () => call<UnitCategory[]>('conversion_units'),

  /** Constants and units that fit `query`, best first. */
  searchLibrary: (query: string, limit?: number) => call<LibraryHit[]>('search_library', { request: { query, limit } }),

  sessions: () => call<SessionInfo[]>('list_sessions'),

  /** A new session, named after the local time. */
  newSession: () =>
    call<SessionInfo>('new_session', { request: { now: Date.now(), utcOffsetMinutes: -new Date().getTimezoneOffset() } }),

  /** Marks a session as used now, so the next start opens it. */
  openSession: (id: string) => call<SessionInfo>('open_session', { id, now: Date.now() }),

  renameSession: (id: string, name: string) => call<SessionInfo>('rename_session', { id, name }),

  duplicateSession: (id: string) => call<SessionInfo>('duplicate_session', { id, now: Date.now() }),

  deleteSession: (id: string) => call<boolean>('delete_session', { id }),

  history: () => call<HistoryEntry[]>('history', { sessionId }),

  clearHistory: () => call<void>('clear_history', { sessionId }),

  /** What the modes of the open session hold. */
  modeInputs: () => call<ModeInputs>('get_mode_inputs', { sessionId }),

  saveModeInputs: (inputs: ModeInputs) => call<void>('set_mode_inputs', { sessionId, inputs }),

  solveCircuit: (request: CircuitRequest) => call<CircuitResponse>('solve_circuit', { request }),

  /** A resistor network by the junction and loop rules, with all the working. */
  solveKirchhoff: (request: KirchhoffRequest) => call<Kirchhoff>('solve_kirchhoff', { request }),

  /** Builds a Geometry scene and answers its task, with the steps at the detail setting. */
  geometry: (request: GeometryRequest) => call<Geometry>('geometry', { request }),
  /** The scene projected for the 3D view. */
  geometryView: (request: GeometryViewRequest) => call<GeometryViewResponse>('geometry_view', { request }),

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

  variables: () => call<Variable[]>('list_variables', { sessionId }),

  /** The session's history written up as given, wanted and solution. */
  writeup: () => call<Writeup>('writeup', { sessionId }),

  forget: (key: string) => call<void>('delete_variable', { key, sessionId }),

  settings: () => call<Settings>('get_settings'),

  /** Checks and saves; returns what the engine now uses. */
  saveSettings: (settings: Settings) => call<Settings>('set_settings', { settings }),
};
