/** The backend contract (spec §5) as the renderer sees it. */

export type StepKind = 'symbolic' | 'substituted' | 'result';

export interface Step {
  kind: StepKind;
  latex: string;
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
}

export interface Health {
  status: string;
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

/** One session per running app; the backend keeps its variables for as long as it lives. */
const SESSION_ID = 'desktop';

const baseUrl = window.quantus?.backendUrl ?? 'http://127.0.0.1:8437';

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  let response: Response;
  try {
    response = await fetch(`${baseUrl}${path}`, {
      ...init,
      headers: { 'Content-Type': 'application/json', ...(init?.headers ?? {}) },
    });
  } catch {
    throw new ApiError('Keine Verbindung zum Backend. Läuft der Kotlin-Dienst?');
  }

  if (response.status === 204) return undefined as T;

  const body = await response.json().catch(() => null);
  if (!response.ok) {
    const error = body as { error?: string; position?: number | null } | null;
    throw new ApiError(error?.error ?? `Der Dienst antwortete mit ${response.status}.`, error?.position ?? null);
  }
  return body as T;
}

export const api = {
  health: () => request<Health>('/api/health'),

  evaluate: (latex: string) =>
    request<Calculation>('/api/evaluate', {
      method: 'POST',
      body: JSON.stringify({ latex, sessionId: SESSION_ID }),
    }),

  variables: () => request<Variable[]>(`/api/variables?sessionId=${SESSION_ID}`),

  forget: (key: string) =>
    request<void>(`/api/variables/${encodeURIComponent(key)}?sessionId=${SESSION_ID}`, { method: 'DELETE' }),
};
