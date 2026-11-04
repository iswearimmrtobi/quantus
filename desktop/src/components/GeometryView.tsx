import { useCallback, useEffect, useRef, useState } from 'react';
import { api, ApiError, type Geometry, type Settings } from '../api';
import { geometryRequest, withTyped, type GeometryInput } from '../geometry';
import { useT } from '../i18n';
import { asError, ErrorRow } from './AnalysisView';
import { GeometryObjects } from './GeometryObjects';
import { GeometryPlot } from './GeometryPlot';
import { GeometryTaskPanel } from './GeometryTask';
import { MathField } from './MathField';


interface Props {
  settings: Settings;
  input: GeometryInput;
  onInputChange(input: GeometryInput): void;
  /** An example for the typed field. */
  placeholder?: string;
}

/**
 * The Geometry mode: a scene of points, vectors, lines, planes, spheres and circles, drawn in a
 * 3D view, and a task on them answered exactly with its steps.
 */
export function GeometryView({ settings, input, onInputChange, placeholder }: Props) {
  const { t } = useT();
  /** The typed line not yet entered. */
  const [typed, setTyped] = useState('');
  const enterTyped = () => {
    if (!typed.trim()) return;
    onInputChange({ ...input, objects: withTyped(input.objects, typed) });
    setTyped('');
  };
  const [answer, setAnswer] = useState<Geometry | null>(null);
  const [error, setError] = useState<ApiError | null>(null);
  /** The latest request; an answer to an older one is dropped. */
  const request = useRef(0);

  const run = useCallback(async (current: GeometryInput) => {
    const id = ++request.current;
    const next = geometryRequest(current);
    if (!next) {
      setAnswer(null);
      setError(null);
      return;
    }
    try {
      const result = await api.geometry(next);
      if (id !== request.current) return;
      setAnswer(result);
      setError(null);
    } catch (failure) {
      if (id !== request.current) return;
      setAnswer(null);
      setError(asError(failure));
    }
  }, []);

  // On opening the page, on every change and whenever a setting changes how the scene is written.
  const { language, precision, stepDetail, angleUnit } = settings;
  useEffect(() => {
    void run(input);
  }, [input, language, precision, stepDetail, angleUnit, run]);

  return (
    <main className="workspace analysis-workspace">
      <div className="analysis-panel geometry-panel">
        <div className="input-row geometry-typed" title={t('geometry.typed.hint')}>
          <MathField value={typed} onChange={setTyped} onEnter={enterTyped} placeholder={placeholder} />
        </div>
        {input.objects.length > 0 && <GeometryPlot objects={input.objects} />}
        {error && <ErrorRow error={error} soft />}
        {!input.objects.length && <p className="analysis-empty">{t('geometry.empty')}</p>}
        <GeometryObjects objects={input.objects} built={answer?.objects ?? null} onChange={(objects) => onInputChange({ ...input, objects })} />
        {input.objects.length > 0 && (
          <GeometryTaskPanel task={input.task} built={answer?.objects ?? null} answer={answer?.task ?? null} onChange={(task) => onInputChange({ ...input, task })} />
        )}
      </div>
    </main>
  );
}
