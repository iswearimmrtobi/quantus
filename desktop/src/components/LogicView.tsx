import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react';
import { api, ApiError, type LogicScene, type Settings } from '../api';
import { addOutput, logicAnswer, logicGrid, logicRequest, nameLatex, shownOutputs, type LogicAnswer, type TermHover } from '../logic';
import { useT } from '../i18n';
import { asError, ErrorRow } from './AnalysisView';
import { Katex } from './Katex';
import { KvMap } from './KvMap';
import { LogicCircuit } from './LogicCircuit';
import { LogicLines } from './LogicLines';
import { LogicResults } from './LogicResults';
import { LogicTable } from './LogicTable';

interface Props {
  settings: Settings;
  scene: LogicScene;
  onSceneChange(scene: LogicScene): void;
  /** Load and Save of the scene as a file. */
  files?: ReactNode;
}

/**
 * The Logic mode (design/digital-logic.md §10): function lines, the truth table beside the KV diagram, and the results
 * of each output, worked out by the core on every change.
 */
export function LogicView({ settings, scene, onSceneChange, files }: Props) {
  const { t } = useT();
  const [answer, setAnswer] = useState<LogicAnswer | null>(null);
  const [failure, setFailure] = useState<ApiError | null>(null);
  /** The output the KV diagram and the results show, by name. */
  const [tab, setTab] = useState<string | null>(null);
  /** The term whose block and formula stand out, in the KV diagram and the results. */
  const [hover, setHover] = useState<TermHover | null>(null);
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
  const { language, precision, stepDetail, logicNotation } = settings;
  useEffect(() => {
    void run(scene);
  }, [scene, language, precision, stepDetail, logicNotation, run]);

  // The circuit's errors name its parts and are shown in the editor; the others above.
  const general = answer?.errors.filter((error) => error.output === null && !error.elements.length) ?? [];

  // The simulation (§10.3): the switches set to a row of the table, every net's value from the core.
  const circuit = answer?.response.circuit ?? null;
  const inputs = answer?.response.inputs ?? [];
  const row = Math.min(scene.row ?? 0, Math.max(0, (1 << inputs.length) - 1));
  const setRow = (next: number) => onSceneChange({ ...scene, row: next });
  const switchValue = (name: string) => {
    const k = inputs.indexOf(name);
    return k < 0 ? null : ((row >> (inputs.length - 1 - k)) & 1) === 1;
  };
  const toggle = (name: string) => {
    const k = inputs.indexOf(name);
    if (k >= 0) setRow(row ^ (1 << (inputs.length - 1 - k)));
  };
  const simulated = circuit ? { currentRow: row, onRow: setRow } : {};
  const grid = logicGrid(scene, answer);
  const outputs = shownOutputs(answer);
  const shown = outputs.find((o) => o.result.name === tab) ?? outputs[0];
  return (
    <main className="workspace analysis-workspace">
      <div className="analysis-panel logic-panel">
        {files}
        {failure && <ErrorRow error={failure} />}
        {general.map((error, i) => (
          <ErrorRow key={i} error={new ApiError(error.message)} />
        ))}
        <LogicLines scene={scene} answer={answer} onChange={onSceneChange} />
        <LogicCircuit settings={settings} scene={scene} answer={answer} onSceneChange={onSceneChange} switchValue={switchValue} onToggle={toggle} />
        {!answer && !failure && <p className="analysis-empty">{t('logic.empty')}</p>}
        {outputs.length > 1 && (
          <div className="form-row logic-tabs">
            {outputs.map(({ result }) => (
              <button key={result.name} className={result.name === shown?.result.name ? 'form-chip active' : 'form-chip'} onClick={() => setTab(result.name)}>
                <Katex latex={nameLatex(result.name)} />
              </button>
            ))}
          </div>
        )}
        <div className="logic-views">
          {grid.columns.length > 0 ? (
            <LogicTable scene={scene} grid={grid} answer={answer} zeros={settings.logicZeros} onChange={onSceneChange} {...simulated} />
          ) : (
            <button className="palette-item logic-table-new" onClick={() => onSceneChange(addOutput(scene, grid))}>
              + {t('logic.table.new')}
            </button>
          )}
          {shown && <KvMap scene={scene} grid={grid} shown={shown} zeros={settings.logicZeros} hover={hover} onHover={setHover} onChange={onSceneChange} {...simulated} />}
        </div>
        {shown && (
          <LogicResults
            scene={scene}
            shown={shown}
            hideSteps={stepDetail === 'none'}
            notation={logicNotation}
            hover={hover}
            onHover={setHover}
            onChange={onSceneChange}
          />
        )}
      </div>
    </main>
  );
}
