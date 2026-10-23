import { invoke } from '@tauri-apps/api/core';

/** The engine contract (spec §5) as the renderer sees it. */

export type StepKind = 'symbolic' | 'substituted' | 'result' | 'algebraic';

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
    notes: string[];
  }[];
  /** The parameter found; the values above are those with it. */
  reverse: { element: string; parameter: ReverseParameter; value: CircuitValue } | null;
}

export type BinomialForm = 'auto' | 'expanded' | 'binomial';
export type Mode = 'algebra' | 'physics' | 'imaginary' | 'electronics';
export type DenominatorForm = 'auto' | 'expanded' | 'factored';

/** What `quantus_core::settings::Settings` serialises to. */
export interface Settings {
  mode: Mode;
  precision: number;
  binomialForm: BinomialForm;
  completeSquare: boolean;
  denominatorForm: DenominatorForm;
  exactRoots: boolean;
  aAndYAsVariables: boolean;
  letterProducts: boolean;
  extraUnitLetters: boolean;
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
      throw new ApiError(error ?? 'Unbekannter Fehler.', position ?? null);
    }
    throw new ApiError(String(failure));
  }
}

export const api = {
  info: () => call<EngineInfo>('engine_info'),

  evaluate: (latex: string) => call<Calculation>('evaluate', { request: { latex, sessionId: SESSION_ID } }),

  solveCircuit: (request: CircuitRequest) => call<CircuitResponse>('solve_circuit', { request }),

  variables: () => call<Variable[]>('list_variables', { sessionId: SESSION_ID }),

  forget: (key: string) => call<void>('delete_variable', { key, sessionId: SESSION_ID }),

  settings: () => call<Settings>('get_settings'),

  /** Checks and saves; returns what the engine now uses. */
  saveSettings: (settings: Settings) => call<Settings>('set_settings', { settings }),
};
