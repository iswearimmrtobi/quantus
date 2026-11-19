import { useMemo, type ReactNode } from 'react';
import type { LogicError, LogicSymbols } from '../api';
import { useT } from '../i18n';
import { SchematicEditor } from '../schematic/Editor';
import { pointKey, wireEnds, type Point } from '../schematic/geometry';
import type { Fullscreen } from '../schematic/view';
import { numberNets, partPins, type LogicPart } from './circuit';
import { LogicInspector, logicKit, logicLabels } from './kit';

interface Props {
  parts: LogicPart[];
  onChange(parts: LogicPart[]): void;
  /** IEC rectangles or ANSI shapes. */
  symbols: LogicSymbols;
  /** Controls in a bar above the canvas: Draw from and Arrange. */
  toolbar?: ReactNode;
  /** Shown in the middle of the canvas while it is empty. */
  emptyCanvas?: ReactNode;
  /** The circuit's errors, each with the parts it concerns, marked red. */
  errors?: LogicError[];
  /** Its warnings (a gate that drives nothing), marked amber. */
  warnings?: LogicError[];
  /** The simulation (§10.3): each net's value in the row the switches are set to, from the core. */
  values?: Map<number, boolean> | null;
  /** A switch's value, by its name; null when the core has no row for it. */
  switchValue?(name: string): boolean | null;
  /** Flips the switch of an input. */
  onToggle?(name: string): void;
  /** Shown in the bar under the canvas: the gate count and the delay. */
  corner?: ReactNode;
  /** A note in the bar under the canvas when there is no error. */
  note?: ReactNode;
  /** The ids along the critical path, input to lamp: they and the nets between them stand out. */
  path?: string[];
  /** Changed to show the whole drawing, as after it was drawn or arranged anew. */
  fitKey?: number;
  /** The full-screen switch, in the zoom controls. */
  fullscreen?: Fullscreen;
}



/**
 * The Logic mode's circuit editor (design/digital-logic.md §10.2): switches, lamps, constants and
 * gates on the grid of the schematic editors, joined by wires; a gate's number of inputs is set in
 * the inspector. The simulation colours the nets, the critical path stands out, errors are red.
 */
export function LogicEditor({ parts, onChange, symbols, toolbar, emptyCanvas, errors = [], warnings = [], values, switchValue, onToggle, corner, note, path = [], fitKey = 0, fullscreen }: Props) {
  const { t } = useT();
  const kit = useMemo(() => logicKit(symbols), [symbols]);
  const labels = useMemo(() => logicLabels(parts), [parts]);

  // The value on a point's net: what the wires and lamps show.
  const nets = useMemo(() => numberNets(parts), [parts]);
  const valueAt = (point: Point): boolean | null => {
    const net = nets.byPoint.get(pointKey(point));
    return net === undefined ? null : (values?.get(net) ?? null);
  };
  // The critical path: its parts, and the nets each of them but the lamp drives.
  const onPath = useMemo(() => new Set(path), [path]);
  const pathNets = useMemo(() => {
    const driven = new Set<number>();
    for (const id of path.slice(0, -1)) {
      const output = parts.find((p) => p.id === id);
      const pin = output && partPins(output).output;
      const net = pin && nets.byPoint.get(pointKey(pin));
      if (net !== undefined && net !== null) driven.add(net);
    }
    return driven;
  }, [path, parts, nets]);
  const wireClass = (wire: LogicPart) => {
    const [a] = wireEnds(wire);
    const value = valueAt(a);
    const net = nets.byPoint.get(pointKey(a));
    return [value === null ? undefined : value ? 'on' : 'off', net !== undefined && pathNets.has(net) ? 'critical' : undefined].filter(Boolean).join(' ') || undefined;
  };

  const faulty = useMemo(() => new Set(errors.flatMap((e) => e.elements)), [errors]);
  const doubtful = useMemo(() => new Set(warnings.flatMap((e) => e.elements)), [warnings]);
  const partClass = (part: LogicPart) =>
    [faulty.has(part.id) ? 'error' : doubtful.has(part.id) ? 'warning' : null, onPath.has(part.id) && 'critical'].filter(Boolean).join(' ') || undefined;
  const notes = errors.length ? errors : warnings;

  /** A switch's value, or the value a lamp shows; null when not known. */
  const simulated = (part: LogicPart): boolean | null => {
    if (part.kind === 'input') return part.name ? (switchValue?.(part.name) ?? null) : null;
    if (part.kind === 'output') return valueAt(partPins(part).inputs[0]);
    return null;
  };

  return (
    <SchematicEditor
      kit={kit}
      elements={parts}
      onChange={onChange}
      labels={labels}
      className="logic-editor"
      hint={t('logic.editor.hint')}
      toolbar={toolbar}
      emptyCanvas={emptyCanvas}
      status={{
        corner,
        note: (notes.length > 0 || note) && (
          <>
            {notes.map((issue, k) => (
              <div key={k}>{issue.message}</div>
            ))}
            {!errors.length && note}
          </>
        ),
        error: errors.length > 0,
      }}
      fitKey={fitKey}
      fullscreen={fullscreen}
      partClass={partClass}
      wireClass={wireClass}
      partState={simulated}
      onTap={(part) => {
        if (part.kind === 'input' && part.name) onToggle?.(part.name);
      }}
      inspector={(part, actions) => (
        <LogicInspector part={part} parts={parts} onChange={actions.update} onReplace={actions.replace} onRemove={actions.remove} onClose={actions.close} />
      )}
    />
  );
}
