import { useEffect, useRef, useState, type ReactNode } from 'react';
import type { DrawForm, LogicScene, LogicTask, Settings } from '../api';
import { useT, type TextKey } from '../i18n';
import { logicTask, nameLatex, type LogicAnswer } from '../logic';
import { drawnFrom, editorData, fromGenerated, toNetlist, withGenerated, withParts, type Generated, type LogicPart } from '../logic/circuit';
import { Katex } from './Katex';
import { LogicEditor } from '../logic/LogicEditor';
import { useFullscreen } from '../schematic/view';

interface Props {
  settings: Settings;
  scene: LogicScene;
  answer: LogicAnswer | null;
  onSceneChange(scene: LogicScene): void;
  /** A switch's value in the simulated row; null when the core has no row for it. */
  switchValue(name: string): boolean | null;
  onToggle(name: string): void;
  /** A drawing asked for elsewhere (the Gates card's **Open in editor**, an output tab); a new object each time. */
  drawRequest?: DrawRequest | null;
  /** The output the output tabs chose for **Draw from**; null for all of them. */
  drawOutput: string | null;
  /** The output tabs, shown in the editor's toolbar while it fills the window. */
  outputTabs?: ReactNode;
  /** Whether the Circuit tab is the one shown; hidden, the circuit is still drawn again when its function changes. */
  shown: boolean;
  /** Brings the Circuit tab up, after a drawing asked for by hand. */
  onShow(): void;
}

/** A circuit to draw: `form` of `output`, rewritten from the expression or not. */
export interface DrawRequest {
  form: DrawForm;
  output: string | null;
  fromExpression: boolean;
  /** Drawn without bringing the Circuit tab up or scrolling to it, as for an output tab. */
  quiet?: boolean;
}

const FORMS: { form: DrawForm; label: TextKey }[] = [
  { form: 'dnf', label: 'logic.draw.dnf' },
  { form: 'knf', label: 'logic.draw.knf' },
  { form: 'nand', label: 'logic.draw.nand' },
  { form: 'nand2', label: 'logic.draw.nand2' },
  { form: 'nor', label: 'logic.draw.nor' },
  { form: 'nor2', label: 'logic.draw.nor2' },
];

/**
 * The circuit panel of the Logic mode (design/digital-logic.md §10.1): the editor with **Draw
 * from…** and **Arrange**, the gate count and delay in its corner, the critical path, and the
 * note when the circuit is out of date. A circuit drawn from a form is drawn again when the
 * function changes, until it is edited (§5.1).
 */
export function LogicCircuit({ settings, scene, answer, onSceneChange, switchValue, onToggle, drawRequest, drawOutput, outputTabs, shown, onShow }: Props) {
  const { t } = useT();
  const data = editorData(scene);
  const parts = data.parts;
  const [taskError, setTaskError] = useState<string | null>(null);
  /** Whether the critical path is marked in the circuit; off until its button turns it on. */
  const [showPath, setShowPath] = useState(false);
  /** Whether the editor fills the window. */
  const fullscreen = useFullscreen();
  const full = fullscreen.full;
  // Hidden (the other tab), it is no longer full screen.
  useEffect(() => {
    if (!shown) fullscreen.setFull(false);
  }, [shown]);
  /** Counts the circuits drawn or arranged anew, each shown whole. */
  const [fitKey, setFitKey] = useState(0);
  /** The scene as it is now, for answers that arrive after it changed. */
  const latest = useRef(scene);
  latest.current = scene;

  const circuit = answer?.response.circuit ?? null;
  const errors = answer?.errors.filter((error) => error.elements.length > 0) ?? [];
  const outputs = answer?.response.outputs.map((o) => o.name) ?? [];

  // An edit makes the circuit hand-drawn: no longer drawn again from the forms.
  const setParts = (next: LogicPart[]) => onSceneChange(withParts(scene, next, scene.outputs.map((_, i) => answer?.outputs[i]?.name)));

  /** The circuit the core gives for `task` over `current`; null, with the error shown, when it gives none. */
  const task = async (current: LogicScene, request: LogicTask): Promise<Generated | null> => {
    const { circuit, error } = await logicTask(current, request);
    setTaskError(error);
    return circuit;
  };

  /** Draws `form` of `output`; `reveal` brings the Circuit tab up, as for a drawing asked for by hand. */
  const draw = async (form: DrawForm, output: string | null, fromExpression = false, reveal = true) => {
    const from = latest.current;
    const drawn = await task(from, { kind: 'draw', form, output, fromExpression });
    if (!drawn) return;
    // Drawn from the scene as it was asked; a change since makes it be drawn again.
    onSceneChange(withGenerated(latest.current, drawn, { form, output, fromExpression }, drawnFrom(from)));
    if (reveal) onShow();
    setFitKey((k) => k + 1);
  };

  // **Open in editor** of the Gates card: drawn here, and the editor brought into view.
  const frame = useRef<HTMLElement>(null);
  useEffect(() => {
    if (!drawRequest) return;
    if (drawRequest.quiet) {
      void draw(drawRequest.form, drawRequest.output, drawRequest.fromExpression, false);
      return;
    }
    // Once the Circuit tab is up, in the next frame.
    void draw(drawRequest.form, drawRequest.output, drawRequest.fromExpression).then(() =>
      requestAnimationFrame(() => frame.current?.scrollIntoView({ behavior: 'smooth', block: 'start' })),
    );
  }, [drawRequest]);

  const arrange = async () => {
    const arranged = await task(latest.current, { kind: 'arrange' });
    if (!arranged) return;
    const current = latest.current;
    const next = fromGenerated(arranged, editorData(current).parts);
    onSceneChange({ ...current, editor: { ...editorData(current), parts: next }, circuit: toNetlist(next) });
    setFitKey((k) => k + 1);
  };

  // A drawn circuit, never edited, follows its function: drawn again once that changed.
  const tried = useRef<string | null>(null);
  const from = drawnFrom(scene);
  const generated = data.generated;
  useEffect(() => {
    if (!generated || generated.from === from || tried.current === from) return;
    tried.current = from;
    // `draw` reads the latest scene itself.
    void draw(generated.form, generated.output, generated.fromExpression, false);
  }, [generated, from]);

  // The critical path, input to lamp, with the names of switches and lamps.
  const named = new Map(parts.filter((p) => p.name).map((p) => [p.id, p.name!]));
  const path = circuit?.criticalPath ?? [];
  const critical = circuit?.outputs.find((o) => o.name === circuit.critical);
  const levels = Math.max(0, ...(circuit?.outputs.map((o) => o.levels) ?? []));
  const outOfDate = !generated ? (circuit?.outOfDate.filter((o) => o.rows.length) ?? []) : [];
  const generatedForm = generated && FORMS.find((f) => f.form === generated.form);
  const setTime = (common: string) => onSceneChange({ ...scene, gateTimes: { ...scene.gateTimes, common } });

  /** The six forms as buttons, drawing for the output chosen or all. */
  const formButtons = (className: string) =>
    FORMS.map(({ form, label }) => (
      <button key={form} className={className} title={t('logic.draw.hint')} disabled={!outputs.length} onClick={() => void draw(form, drawOutput && outputs.includes(drawOutput) ? drawOutput : null)}>
        {t(label)}
      </button>
    ));

  const toolbar = (
    <div className="logic-draw">
      <span className="palette-title">{t('logic.draw.title')}</span>
      <div className="logic-draw-forms">{formButtons('form-chip')}</div>
      <button className="form-chip logic-arrange" title={t('logic.arrange.hint')} disabled={!circuit} onClick={() => void arrange()}>
        {t('logic.arrange')}
      </button>
      {full && outputTabs}
      {taskError && <p className="file-error">{taskError}</p>}
    </div>
  );

  // On an empty canvas: the forms to draw from, or how to start without a function.
  const emptyCanvas = outputs.length ? (
    <div className="logic-canvas-start">
      <span className="palette-title">{t('logic.draw.title')}</span>
      <div className="logic-draw-forms">{formButtons('form-chip')}</div>
    </div>
  ) : (
    <p className="panel-empty">{t('logic.editor.empty')}</p>
  );

  const corner = circuit && (
    <div className="logic-figures">
      <div>{t('logic.gates', { gates: circuit.gates, inputs: circuit.gateInputs, levels })}</div>
      {critical && (
        <div className="logic-delay">
          {t('logic.delay')}{' '}
          {critical.time ? <Katex latex={critical.time} /> : <Katex latex={`${critical.levels}\\,t_{pd}`} />}
        </div>
      )}
      {path.length > 1 && (
        <button className={showPath ? 'logic-path active' : 'logic-path'} title={t('logic.delay.path.hint')} aria-pressed={showPath} onClick={() => setShowPath(!showPath)}>
          {path.map((id, k) => (
            <span key={k}>
              {k > 0 && ' → '}
              {named.has(id) ? <Katex latex={nameLatex(named.get(id)!)} /> : id}
            </span>
          ))}
        </button>
      )}
      <label className="logic-tpd" title={t('logic.delay.tpd.hint')}>
        <Katex latex="t_{pd} =" />
        <input type="text" spellCheck={false} autoComplete="off" placeholder="10ns" value={scene.gateTimes?.common ?? ''} onChange={(event) => setTime(event.target.value)} />
      </label>
    </div>
  );

  const note =
    outOfDate.length > 0 ? (
      <>
        {outOfDate.map((o) => (
          <div key={o.output}>{t('logic.outOfDate', { output: o.output, rows: o.rows.join(', ') })}</div>
        ))}
        <button className="form-chip" onClick={() => void draw('dnf', null)}>
          {t('logic.outOfDate.draw')}
        </button>
      </>
    ) : generatedForm ? (
      t('logic.generated', { form: t(generatedForm.label) })
    ) : null;

  return (
    <section className="logic-circuit" ref={frame} hidden={!shown}>
      {shown && (
        <div className="logic-editor-frame">
          <LogicEditor
            parts={parts}
            onChange={setParts}
            symbols={settings.logicSymbols}
            toolbar={toolbar}
            emptyCanvas={emptyCanvas}
            errors={errors}
            warnings={circuit?.warnings ?? []}
            values={circuit ? new Map(circuit.values.map((v) => [v.net, v.value])) : null}
            switchValue={switchValue}
            onToggle={onToggle}
            corner={corner}
            note={note}
            path={showPath ? path : []}
            fitKey={fitKey}
            fullscreen={fullscreen}
          />
        </div>
      )}
    </section>
  );
}
