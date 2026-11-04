import { useCallback, useEffect, useRef, useState } from 'react';
import { api, ApiError, type Geometry, type Settings } from '../api';
import { geometryRequest, type GeometryInput } from '../geometry';
import { useT } from '../i18n';
import { asError, ErrorRow } from './AnalysisView';
import { GeometryPlot } from './GeometryPlot';
import { Katex } from './Katex';


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
export function GeometryView({ settings, input }: Props) {
  const { t } = useT();
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
        {input.objects.length > 0 && <GeometryPlot objects={input.objects} />}
        {error && <ErrorRow error={error} soft />}
        {!input.objects.length && <p className="analysis-empty">{t('geometry.empty')}</p>}
        {answer && answer.objects.length > 0 && (
          <section className="section-card geometry-objects">
            {answer.objects.map((object, index) => (
              <div key={index} className="geometry-object">
                {object.error ? <span className="geometry-error">{object.name}: {object.error}</span> : <Katex latex={object.latex} />}
              </div>
            ))}
          </section>
        )}
      </div>
    </main>
  );
}
