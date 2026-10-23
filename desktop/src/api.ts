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
  unit: string;
  quantityName: string | null;
  dimension: string;
  variable: Variable | null;
  domainRestrictions?: string[];
  warning?: string;
}

export type BinomialForm = 'auto' | 'expanded' | 'binomial';
export type DenominatorForm = 'auto' | 'expanded' | 'factored';

/** What `quantus_core::settings::Settings` serialises to. */
export interface Settings {
  precision: number;
  binomialForm: BinomialForm;
  completeSquare: boolean;
  denominatorForm: DenominatorForm;
  aAndYAsVariables: boolean;
  letterProducts: boolean;
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

  variables: () => call<Variable[]>('list_variables', { sessionId: SESSION_ID }),

  forget: (key: string) => call<void>('delete_variable', { key, sessionId: SESSION_ID }),

  settings: () => call<Settings>('get_settings'),

  /** Checks and saves; returns what the engine now uses. */
  saveSettings: (settings: Settings) => call<Settings>('set_settings', { settings }),
};
