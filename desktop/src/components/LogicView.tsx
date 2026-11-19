import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react';
import { api, ApiError, type LogicScene, type Settings } from '../api';
import { addOutput, logicAnswer, logicGrid, logicRequest, nameLatex, shownOutputs, type LogicAnswer, type TermHover } from '../logic';
import { useT } from '../i18n';
import { asError, ErrorRow } from './AnalysisView';
import { Katex } from './Katex';
import { KvMap } from './KvMap';
import { LogicCircuit, type DrawRequest } from './LogicCircuit';
import { LogicLines } from './LogicLines';
import { LogicResults } from './LogicResults';
import { LogicTable } from './LogicTable';
import { Segmented } from './SettingsDialog';
import { editorData } from '../logic/circuit';

/** The two examples of the empty page: an expression and a minterm list. */
const EXAMPLE_EXPRESSION = 'f = A \\land B \\lor C';
const EXAMPLE_MINTERMS = 'f = \\sum m(3, 5, 6, 7)';

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
  /** A circuit to draw in the editor, asked for by the Gates card. */
  const [drawRequest, setDrawRequest] = useState<DrawRequest | null>(null);
  /** The circuit tab's output for **Draw from**, null for all; a drawn circuit's own output wins. */
  const [drawScope, setDrawScope] = useState<string | null>(null);
  /** The tab over the table and the KV diagram, or over the circuit editor; the circuit's at first when there is one. */
  const [view, setView] = useState<'table' | 'circuit'>(() => (editorData(scene).parts.length ? 'circuit' : 'table'));

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
  // In the circuit view the output tabs say which outputs the circuit is drawn for: All, or one.
  // A circuit drawn from a form shows its own; a gone output falls back to All.
  const generated = editorData(scene).generated;
  const known = (name: string | null) => (name !== null && outputs.some((o) => o.result.name === name) ? name : null);
  const drawOutput = known(generated ? generated.output : drawScope);
  // A drawing keeps its output as the tab's choice once the circuit is edited by hand.
  const generatedOutput = generated ? generated.output : undefined;
  useEffect(() => {
    if (generatedOutput !== undefined) setDrawScope(generatedOutput);
  }, [generatedOutput]);
  const chooseDrawOutput = (name: string | null) => {
    setDrawScope(name);
    if (name !== null) setTab(name);
    // A drawn circuit is drawn again for the output at once; a hand-made one is left as it is.
    if (generated && generated.output !== name) setDrawRequest({ form: generated.form, output: name, fromExpression: generated.fromExpression ?? false, quiet: true });
  };
  // The circuit view's output tabs, also shown in the editor's toolbar while it fills the window.
  const outputTabs =
    outputs.length > 1 ? (
      <div className="form-row logic-output-tabs" role="group" title={t('logic.draw.output')}>
        <button className={drawOutput === null ? 'form-chip active' : 'form-chip'} aria-pressed={drawOutput === null} onClick={() => chooseDrawOutput(null)}>
          {t('logic.draw.all')}
        </button>
        {outputs.map(({ result }) => (
          <button
            key={result.name}
            className={result.name === drawOutput ? 'form-chip active' : 'form-chip'}
            aria-pressed={result.name === drawOutput}
            onClick={() => chooseDrawOutput(result.name)}
          >
            <Katex latex={nameLatex(result.name)} />
          </button>
        ))}
      </div>
    ) : null;
  return (
    <main className="workspace analysis-workspace">
      <div className="analysis-panel logic-panel">
        {files}
        {failure && <ErrorRow error={failure} />}
        {general.map((error, i) => (
          <ErrorRow key={i} error={new ApiError(error.message)} />
        ))}
        <LogicLines scene={scene} answer={answer} onChange={onSceneChange} />
        {!answer && !failure && (
          <p className="analysis-empty logic-empty">
            {t('logic.empty.lead')} <Katex latex={EXAMPLE_EXPRESSION} /> {t('logic.empty.or')} <Katex latex={EXAMPLE_MINTERMS} />
            {t('logic.empty.tail')}
          </p>
        )}
        <div className="logic-tabs">
          <Segmented
            choices={[
              { value: 'table', label: t('logic.tab.table') },
              { value: 'circuit', label: t('logic.tab.circuit') },
            ]}
            value={view}
            onChange={setView}
          />
          {outputs.length > 1 && view === 'table' && (
            <div className="form-row logic-output-tabs">
              {outputs.map(({ result }) => (
                <button key={result.name} className={result.name === shown?.result.name ? 'form-chip active' : 'form-chip'} onClick={() => setTab(result.name)}>
                  <Katex latex={nameLatex(result.name)} />
                </button>
              ))}
            </div>
          )}
          {view === 'circuit' && outputTabs}
        </div>
        <LogicCircuit
          settings={settings}
          scene={scene}
          answer={answer}
          onSceneChange={onSceneChange}
          switchValue={switchValue}
          onToggle={toggle}
          drawRequest={drawRequest}
          drawOutput={drawOutput}
          outputTabs={outputTabs}
          shown={view === 'circuit'}
          onShow={() => setView('circuit')}
        />
        {view === 'table' && (
          <div className="logic-views">
            {grid.columns.length > 0 ? (
              <LogicTable scene={scene} grid={grid} answer={answer} zeros={settings.logicZeros} onChange={onSceneChange} {...simulated} />
            ) : (
              <button className="palette-item logic-table-new" onClick={() => onSceneChange(addOutput(scene, grid))}>
                + {t('logic.table.new')}
              </button>
            )}
            {shown && <KvMap scene={scene} grid={grid} shown={shown} zeros={settings.logicZeros} notation={logicNotation} hover={hover} onHover={setHover} onChange={onSceneChange} {...simulated} />}
          </div>
        )}
        {shown && (
          <LogicResults
            scene={scene}
            shown={shown}
            hideSteps={stepDetail === 'none'}
            notation={logicNotation}
            symbols={settings.logicSymbols}
            onOpen={(form, output, fromExpression) => setDrawRequest({ form, output, fromExpression })}
            several={outputs.length > 1}
            joint={outputs.length > 1 ? (answer?.response.joint ?? null) : null}
            hover={hover}
            onHover={setHover}
            onChange={onSceneChange}
          />
        )}
      </div>
    </main>
  );
}
