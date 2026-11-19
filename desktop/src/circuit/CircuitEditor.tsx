import { useMemo, type ReactNode } from 'react';
import type { CircuitValue } from '../api';
import { useT } from '../i18n';
import { SchematicEditor } from '../schematic/Editor';
import { pointKey, type Rect } from '../schematic/geometry';
import type { CanvasLabel } from '../schematic/labels';
import { CircuitInspector, circuitKit, nodePlace, partPlace, Value, type ElementResult, type Place } from './kit';
import { body, FIELDS, QUANTITIES, type EditorElement, type EditorKind, type Field, type Point, type Shown } from './model';

/** A label a mode adds to the canvas; it is laid out with the part labels and kept clear of them. */
export interface ExtraLabel extends Place {
  id: string;
  className?: string;
  content: ReactNode;
}

/**
 * The schematic editor of Electronics and Kirchhoff. It draws and edits the parts; what is
 * computed from them comes in from the mode that uses it, as do the parts it offers and anything
 * it draws on top.
 */
interface Props {
  elements: EditorElement[];
  onChange(next: EditorElement[]): void;
  /** The kinds of part the palette offers, in order. */
  palette?: EditorKind[];
  /** The inspector's fields per kind; a wire without fields is ideal and has no material choice either. */
  fields?: Record<EditorKind, Field[]>;
  /** U, I, P and R per part, shown on the canvas and in the inspector. */
  results?: ElementResult[];
  /** The potentials, each at one point of its node. */
  voltages?: { point: Point; voltage: CircuitValue }[];
  /** Why there are no results; shown as an error over the canvas. */
  failure?: string | null;
  /** A hint over the canvas when there is no failure. */
  note?: string | null;
  /** SVG drawn over the parts, in drawing coordinates. */
  overlay?: ReactNode;
  /** More labels, after the part labels and before the potentials. */
  labels?: ExtraLabel[];
  /** Places the labels must keep clear of, besides the parts, such as what `overlay` draws. */
  reserved?: Rect[];
  /** Parts drawn emphasised, as those of the loop under the pointer. */
  highlight?: string[];
  /**
   * Another drawing shown in place of `elements` for as long as it is set, such as the network
   * after a merge of resistors. It is only looked at: nothing on the canvas can be edited then.
   */
  preview?: EditorElement[];
  /** Shown under the canvas, between the palette and the inspector. */
  below?: ReactNode;
  /** Which results the parts show on the canvas. */
  shown: Shown;
  onShownChange(next: Shown): void;
  /** Extra controls for the palette column (load and save). */
  toolbar?: ReactNode;
}

export const PALETTE: EditorKind[] = ['voltageSource', 'currentSource', 'resistor', 'potentiometer', 'powerLoad', 'wire', 'ground'];

export function CircuitEditor({
  elements,
  onChange,
  palette = PALETTE,
  fields = FIELDS,
  results: resultList,
  voltages,
  failure,
  note,
  overlay,
  labels: extraLabels,
  reserved,
  highlight,
  preview,
  below,
  shown,
  onShownChange,
  toolbar,
}: Props) {
  const { t } = useT();
  const kit = useMemo(() => circuitKit(palette), [palette]);
  const results = useMemo(() => new Map(resultList?.map((e) => [e.id, e])), [resultList]);

  /**
   * Part labels first, in drawing order, then the mode's own, then the node voltages, which give
   * way to them. A preview shows its parts' labels only.
   */
  const labels = useMemo(() => {
    const list: CanvasLabel[] = [];
    for (const element of preview ?? elements) {
      const place = partPlace(element);
      if (!place) continue;
      const result = results.get(element.id);
      const content = (
        <>
          <span className="part-id">{element.id}</span>
          {result &&
            QUANTITIES.map(({ key, symbol }) => {
              const value = result[key];
              return shown[key] && value ? <Value key={key} label={symbol} value={value} /> : null;
            })}
        </>
      );
      list.push({ ...place, id: element.id, className: 'part-label', content, leaderFrom: body(element) });
    }
    if (preview) return list;
    for (const extra of extraLabels ?? []) list.push({ ...extra, className: extra.className ?? '' });
    for (const { point, voltage } of voltages ?? []) {
      list.push({ ...nodePlace(point), id: `node-${pointKey(point)}`, className: 'node-label', content: <Value label="φ" value={voltage} /> });
    }
    return list;
  }, [elements, preview, results, shown, extraLabels, voltages]);

  const highlighted = useMemo(() => new Set(highlight), [highlight]);

  return (
    <SchematicEditor
      kit={kit}
      elements={elements}
      onChange={onChange}
      labels={labels}
      reserved={reserved}
      overlay={overlay}
      preview={preview}
      partClass={(element) => (highlighted.has(element.id) ? 'highlighted' : undefined)}
      hint={t('circuit.hint')}
      paletteExtras={toolbar}
      emptyCanvas={t('circuit.empty')}
      stageExtras={
        <>
          <div className="segmented circuit-shown" role="group" aria-label={t('circuit.show')} title={t('circuit.show')}>
            {QUANTITIES.map((quantity) => (
              <button
                key={quantity.key}
                className={shown[quantity.key] ? 'segment active' : 'segment'}
                aria-pressed={shown[quantity.key]}
                title={t(quantity.label)}
                onClick={() => onShownChange({ ...shown, [quantity.key]: !shown[quantity.key] })}
              >
                {quantity.symbol}
              </button>
            ))}
          </div>
          {(failure || note) && <div className={failure ? 'circuit-note error' : 'circuit-note'}>{failure ?? note}</div>}
        </>
      }
      below={below}
      inspector={(element, actions) => (
        <CircuitInspector
          element={element}
          fields={fields[element.kind]}
          result={results.get(element.id)}
          onChange={actions.update}
          onRemove={actions.remove}
          onClose={actions.close}
        />
      )}
    />
  );
}
