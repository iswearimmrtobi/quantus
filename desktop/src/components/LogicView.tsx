import { useCallback, useEffect, useRef, useState } from 'react';
import { api, ApiError, type LogicScene, type Settings } from '../api';
import { logicAnswer, logicRequest, type LogicAnswer } from '../logic';
import { useT } from '../i18n';
import { asError, ErrorRow } from './AnalysisView';
import { LogicLines, SourceMark } from './LogicLines';

interface Props {
  settings: Settings;
  scene: LogicScene;
  onSceneChange(scene: LogicScene): void;
}

/**
 * The Logic mode (design/digital-logic.md §10): function lines, the truth table and the results
 * of each output, worked out by the core on every change.
 */
export function LogicView({ settings, scene, onSceneChange }: Props) {
  const { t } = useT();
  const [answer, setAnswer] = useState<LogicAnswer | null>(null);
  const [failure, setFailure] = useState<ApiError | null>(null);
  /** The latest request; an answer to an older one is dropped. */
  const request = useRef(0);

  const run = useCallback(async (current: LogicScene) => {
    const id = ++request.current;
    const sent = logicRequest(current);
    if (!sent.request) {
      setAnswer(null);
      setFailure(null);
      return;
    }
    try {
      const response = await api.logic(sent.request);
      if (id !== request.current) return;
      setAnswer(logicAnswer(current, sent, response));
      setFailure(null);
    } catch (error) {
      if (id !== request.current) return;
      setAnswer(null);
      setFailure(asError(error));
    }
  }, []);

  // On opening the page, on every change and whenever a setting changes how the results are written.
  const { language, precision, stepDetail, logicNotation, logicTwoInput } = settings;
  useEffect(() => {
    void run(scene);
  }, [scene, language, precision, stepDetail, logicNotation, logicTwoInput, run]);

  const general = answer?.errors.filter((error) => error.output === null) ?? [];
  return (
    <main className="workspace analysis-workspace">
      <div className="analysis-panel logic-panel">
        {failure && <ErrorRow error={failure} />}
        {general.map((error, i) => (
          <ErrorRow key={i} error={new ApiError(error.message)} />
        ))}
        <LogicLines scene={scene} answer={answer} onChange={onSceneChange} />
        {!answer && !failure && <p className="analysis-empty">{t('logic.empty')}</p>}
        {answer && answer.extra.length > 0 && (
          <div className="logic-outputs">
            {answer.extra.map((output) => (
              <div className="logic-output-name" key={output.name}>
                <span className="logic-name">{output.name}</span>
                <SourceMark kind={output.source} />
              </div>
            ))}
          </div>
        )}
      </div>
    </main>
  );
}
