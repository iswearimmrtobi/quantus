import { useEffect, useMemo, useRef, type CSSProperties, type PointerEvent as ReactPointerEvent, type ReactNode, type Ref } from 'react';
import type { CircuitResponse, CircuitValue } from '../api';
import {
  body,
  FIELDS,
  connectedPoints,
  GRID,
  HALF,
  junctionDots,
  isDiagonal,
  isIdealWire,
  KIND_LABELS,
  MATERIALS,
  newElement,
  obstacles,
  QUANTITIES,
  pointKey,
  terminals,
  turnedBy,
  type EditorElement,
  type EditorKind,
  type Field,
  type Point,
  type Shown,
  type Terminal,
} from '../circuit/model';
import { boundsOf } from '../schematic/geometry';
import { useT } from '../i18n';
import { nearest, placeLabels, type LabelRequest, type Placement, type Rect, type Side } from '../schematic/labels';
import { Katex } from './Katex';
import { DragGhost, GridBackground, HOME, InspectorHead, InspectorPopover, labelStyle, useCanvasView, useLabelSizes, useSchematicEditing, viewTransform, WirePart, ZoomControls, useFullscreen } from './Schematic';

type ElementResult = CircuitResponse['elements'][number];

/** A label a mode adds to the canvas; it is laid out with the part labels and kept clear of them. */
export interface ExtraLabel extends Place {
  id: string;
  className?: string;
  content: ReactNode;
}

/**
 * The schematic editor. It draws and edits the parts; what is computed from them comes in from
 * the mode that uses it, as do the parts it offers and anything it draws on top.
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

export const PALETTE: EditorKind[] = [
  'voltageSource',
  'currentSource',
  'resistor',
  'potentiometer',
  'powerLoad',
  'wire',
  'ground',
];

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
  const canvas = useRef<SVGSVGElement>(null);
  const canvasView = useCanvasView(canvas);
  const { view, setView, toGrid } = canvasView;
  const pins = (e: EditorElement) => Object.values(terminals(e)) as Point[];
  const editing = useSchematicEditing({
    elements,
    onChange,
    canvas,
    view: canvasView,
    pins,
    newWire: (next, from, to) => ({ ...newElement(next, 'wire', from), x2: to.x, y2: to.y }),
    turned: (rotation) => turnedBy(rotation, 2),
  });
  const { selection, inspected, drag, update, remove } = editing;

  const results = useMemo(() => new Map(resultList?.map((e) => [e.id, e])), [resultList]);
  /** What the canvas shows: the elements, or the preview while there is one. */
  const drawn = preview ?? elements;
  const connected = useMemo(() => connectedPoints(drawn), [drawn]);
  const dots = useMemo(() => junctionDots(drawn), [drawn]);

  const { sizes, measure } = useLabelSizes();

  /**
   * Part labels first, in drawing order, then the mode's own, then the node voltages, which give
   * way to them.
   */
  const labels = useMemo(() => {
    const list: (Place & { id: string; element?: EditorElement; voltage?: CircuitValue; extra?: ExtraLabel })[] = [];
    for (const element of drawn) {
      const place = partPlace(element);
      if (place) list.push({ ...place, id: element.id, element });
    }
    if (preview) return list;
    for (const extra of extraLabels ?? []) list.push({ ...extra, extra });
    for (const { point, voltage } of voltages ?? []) {
      list.push({ ...nodePlace(point), id: `node-${pointKey(point)}`, voltage });
    }
    return list;
  }, [drawn, preview, extraLabels, voltages]);

  // While parts are dragged, every label keeps its spot as long as it stays free, so the labels do
  // not jump about under the pointer; dropping lays them out afresh.
  const dragging = drag?.type === 'move' || drag?.type === 'end';
  const lastPlacements = useRef<Map<string, Placement>>(new Map());
  const placements = useMemo(() => {
    const requests: LabelRequest[] = [];
    for (const label of labels) {
      const size = sizes.get(label.id);
      if (size) requests.push({ ...label, ...size });
    }
    const blocked = [...obstacles(drawn), ...(reserved ?? [])];
    return placeLabels(requests, blocked, dragging ? lastPlacements.current : undefined);
  }, [labels, sizes, drawn, reserved, dragging]);
  useEffect(() => {
    lastPlacements.current = placements;
  }, [placements]);

  /** Shows the whole drawing with its labels; the start view without one. */
  const fitAll = () => (elements.length ? canvasView.fit(boundsOf([...obstacles(elements), ...placements.values()])) : setView(HOME));

  const add = (kind: EditorKind, at?: Point) => {
    // There are two terminals, A and B, and no third.
    if (kind === 'terminal' && elements.filter((e) => e.kind === 'terminal').length >= 2) return;
    // Without a drop position the part goes to a free spot, one row below the last one.
    const spot = at ?? {
      x: (160 - view.x) / view.zoom + (elements.length % 4) * 3 * GRID,
      y: (120 - view.y) / view.zoom + Math.floor(elements.length / 4) * 3 * GRID,
    };
    const element = newElement(elements, kind, spot);
    onChange([...elements, element]);
    editing.inspect(element.id);
    // The canvas takes the keys, so that Del removes the new part at once.
    canvas.current?.focus();
  };

  /** Turns each selected resistor by 45° about its middle; other parts stay as they are. */
  const tiltSelected = () => {
    if (!elements.some((e) => selection.includes(e.id) && e.kind === 'resistor')) return;
    onChange(elements.map((e) => (selection.includes(e.id) && e.kind === 'resistor' ? { ...e, rotation: turnedBy(e.rotation, 1) } : e)));
  };

  const onKeyDown = (event: React.KeyboardEvent) =>
    editing.keyDown(event, {
      home: () => setView(HOME),
      fit: fitAll,
      // Shift+R tilts a resistor.
      extra: (key) => key.key.toLowerCase() === 'r' && key.shiftKey && (tiltSelected(), true),
    });

  const chosen = (!preview && drag?.type !== 'move' && elements.find((e) => e.id === inspected)) || null;
  const fullscreen = useFullscreen();

  return fullscreen.portal(
    <div className={fullscreen.full ? 'circuit fullscreen' : 'circuit'}>
      <div className="circuit-palette">
        <div className="palette-title">{t('circuit.parts')}</div>
        {palette.map((kind) => (
          <button
            key={kind}
            className="palette-item"
            draggable
            title={t('circuit.drag')}
            onDragStart={(event) => event.dataTransfer.setData('text/plain', kind)}
            onClick={() => add(kind)}
          >
            <svg viewBox="-48 -24 96 48" width="48" height="24">
              <Symbol kind={kind} />
            </svg>
            <span>{t(KIND_LABELS[kind])}</span>
          </button>
        ))}
        {toolbar}
        <p className="palette-hint">
          {t('circuit.hint')} <kbd>{t('key.space')}</kbd> {t('circuit.rotates')}, <kbd>{t('key.shiftR')}</kbd> {t('circuit.tilts')}, <kbd>{t('key.delete')}</kbd> {t('circuit.deletes')}.
        </p>
        <p className="palette-hint">
          {t('circuit.pan')} <kbd>{t('key.home')}</kbd> {t('circuit.home')}.
        </p>
      </div>

      <div className="circuit-centre">
      <div className="circuit-stage">
        <ZoomControls view={canvasView} fit={fitAll} fullscreen={fullscreen} />
        <svg
          ref={canvas}
          className="circuit-canvas"
          tabIndex={0}
          onKeyDown={onKeyDown}
          onPointerMove={editing.onMove}
          onPointerUp={editing.onUp}
          onPointerCancel={() => editing.setDrag(null)}
          onPointerDown={editing.startCanvas}
          onContextMenu={(event) => event.preventDefault()}
          onDragOver={(event) => event.preventDefault()}
          onDrop={(event) => {
            event.preventDefault();
            const kind = event.dataTransfer.getData('text/plain') as EditorKind;
            if (palette.includes(kind)) add(kind, toGrid(event));
          }}
        >
          <GridBackground view={view} />

          <g transform={viewTransform(view)}>
            <g pointerEvents={preview ? 'none' : undefined}>
            {drawn.map((element) => (
              <Part
                key={element.id}
                element={element}
                selected={!preview && selection.includes(element.id)}
                highlighted={highlight?.includes(element.id) ?? false}
                connected={connected}
                onPointerDown={(event) => editing.startMove(event, element)}
                onTerminal={(event, point) => editing.startDraw(event, point, element)}
                onEnd={(event, end) => editing.startEnd(event, element, end)}
              />
            ))}
            </g>

            {dots.map((point) => (
              <circle key={pointKey(point)} className="junction-dot" cx={point.x} cy={point.y} r="3.5" />
            ))}

            {!preview && overlay}

            {labels.map((label) => {
              const placement = placements.get(label.id);
              if (!placement?.displaced) return null;
              const centre = { x: placement.x + placement.w / 2, y: placement.y + placement.h / 2 };
              const from = label.element ? nearest(body(label.element), centre) : label.anchor;
              const to = nearest(placement, from);
              return <line key={label.id} className="label-leader" x1={from.x} y1={from.y} x2={to.x} y2={to.y} />;
            })}

            <DragGhost drag={drag} />
          </g>
        </svg>

        {/* The values are HTML over the canvas, not foreignObject in it: WebKitGTK draws KaTeX's
            positioned spans inside a foreignObject at the origin of the SVG. */}
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

        <div className="circuit-labels">
          <div
            className="circuit-labels-drawing"
            style={{ transform: `translate(${view.x}px, ${view.y}px) scale(${view.zoom})` }}
          >
            {labels.map((label) =>
              label.element ? (
                <PartLabel
                  key={label.id}
                  element={label.element}
                  result={results.get(label.element.id)}
                  shown={shown}
                  style={labelStyle(placements.get(label.id), label.anchor)}
                  measure={measure}
                />
              ) : label.extra ? (
                <div
                  key={label.id}
                  className={label.extra.className}
                  style={labelStyle(placements.get(label.id), label.anchor)}
                  ref={measure}
                  data-label={label.id}
                >
                  {label.extra.content}
                </div>
              ) : (
                <div
                  key={label.id}
                  className="node-label"
                  style={labelStyle(placements.get(label.id), label.anchor)}
                  ref={measure}
                  data-label={label.id}
                >
                  <Value label="φ" value={label.voltage!} />
                </div>
              ),
            )}
          </div>
        </div>

        {elements.length === 0 && !preview && <div className="circuit-canvas-empty">{t('circuit.empty')}</div>}

        {(failure || note) && <div className={failure ? 'circuit-note error' : 'circuit-note'}>{failure ?? note}</div>}

        {chosen && (
          <InspectorPopover box={body(chosen)} view={view} canvas={canvas}>
            <Inspector
              element={chosen}
              fields={fields[chosen.kind]}
              result={results.get(chosen.id)}
              onChange={(change) => update(chosen.id, change)}
              onRemove={() => remove([chosen.id])}
              onClose={editing.closeInspector}
            />
          </InspectorPopover>
        )}
      </div>
      {below}
      </div>
    </div>
  );
}

/** The symbol of a kind in its own coordinates: terminals at (±HALF, 0), the wiper at (0, −HALF). */
function Symbol({ kind }: { kind: EditorKind }) {
  switch (kind) {
    case 'resistor':
      return (
        <>
          <path d={`M${-HALF} 0H-20M20 0H${HALF}`} />
          <rect x="-20" y="-7" width="40" height="14" />
        </>
      );
    case 'potentiometer':
      return (
        <>
          <path d={`M${-HALF} 0H-20M20 0H${HALF}M0 ${-HALF}V-14`} />
          <rect x="-20" y="-7" width="40" height="14" />
          <path d="M-4 -14L0 -7L4 -14Z" />
        </>
      );
    case 'voltageSource':
      return (
        <>
          <path d={`M${-HALF} 0H-14M14 0H${HALF}`} />
          <circle r="14" />
          <path d="M-9 -3v-5M-11.5 -5.5h5M5 -5.5h5" />
        </>
      );
    case 'currentSource':
      return (
        <>
          <path d={`M${-HALF} 0H-14M14 0H${HALF}`} />
          <circle r="14" />
          <path d="M-8 0H8M3 -4L8 0L3 4" />
        </>
      );
    case 'powerLoad':
      return (
        <>
          <path d={`M${-HALF} 0H-14M14 0H${HALF}`} />
          <rect x="-14" y="-14" width="28" height="28" />
          <path d="M-8 -8L8 8" />
        </>
      );
    case 'ground':
      return <path d="M0 0V8M-12 8H12M-8 13H8M-4 18H4" />;
    case 'terminal':
      return <circle r="8" />;
    case 'wire':
      return <path d={`M${-HALF} 0H${HALF}`} />;
  }
}

interface PartProps {
  element: EditorElement;
  selected: boolean;
  highlighted: boolean;
  /** The points where terminals meet; their dots are hidden until hovered. */
  connected: Set<string>;
  onPointerDown(event: ReactPointerEvent): void;
  onTerminal(event: ReactPointerEvent, point: Point): void;
  onEnd(event: ReactPointerEvent, end: 'a' | 'b'): void;
}

function Part({ element, selected, highlighted, connected, onPointerDown, onTerminal, onEnd }: PartProps) {
  const places = terminals(element);
  const dot = (base: string, point: Point) => (connected.has(pointKey(point)) ? `${base} connected` : base);
  const className = ['part', selected && 'selected', highlighted && 'highlighted'].filter(Boolean).join(' ');

  if (element.kind === 'wire') {
    const { a, b } = places as Record<'a' | 'b', Point>;
    return (
      <WirePart
        a={a}
        b={b}
        className={className}
        joins={isIdealWire(element)}
        connected={connected}
        onPointerDown={onPointerDown}
        onEnd={onEnd}
        onTerminal={onTerminal}
      />
    );
  }

  return (
    <g className={className}>
      <g transform={`translate(${element.x} ${element.y}) rotate(${element.rotation * 90})`}>
        <g className="symbol">
          <Symbol kind={element.kind} />
          {/* At 45° the terminals are √2 times as far out, so the leads run on to them. */}
          {isDiagonal(element.rotation) && <path d={`M${-HALF * Math.SQRT2} 0H${-HALF}M${HALF} 0H${HALF * Math.SQRT2}`} />}
        </g>
        <rect className="part-hit" x={-HALF + 8} y="-16" width={2 * HALF - 16} height="32" onPointerDown={onPointerDown} />
      </g>
      {(Object.entries(places) as [Terminal, Point][]).map(([name, point]) => (
        <circle
          key={name}
          className={dot('terminal', point)}
          cx={point.x}
          cy={point.y}
          r="5"
          onPointerDown={(event) =>
            element.kind === 'ground' || element.kind === 'terminal' ? onPointerDown(event) : onTerminal(event, point)
          }
        />
      ))}
      {element.kind === 'terminal' && (
        <text className="terminal-name" x={element.x + 12} y={element.y - 10}>
          {element.id}
        </text>
      )}
    </g>
  );
}

export type Place = Pick<LabelRequest, 'anchor' | 'sides' | 'gap' | 'corner'>;

/**
 * Where a part's label would like to sit: under a horizontal part, right of a vertical one, on the
 * other side if a potentiometer's wiper is there; the layout takes the next free spot if that one
 * is not. An ideal wire has no drop, so it only gets a label once it has a material or a
 * resistance; the inspector shows the rest.
 */
function partPlace(element: EditorElement): Place | null {
  if (element.kind === 'ground' || element.kind === 'terminal') return null;
  if (element.kind === 'wire') {
    if (!element.material && !element.rOverride?.trim()) return null;
    const { a, b } = terminals(element) as Record<'a' | 'b', Point>;
    const anchor = { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
    const across = Math.abs(b.x - a.x) >= Math.abs(b.y - a.y);
    return { anchor, gap: 8, sides: across ? ['above', 'below', 'right', 'left'] : ['right', 'left', 'above', 'below'] };
  }
  // A part at 45° counts with the direction it leans more towards: 45° as across, 135° as down.
  const vertical = Math.floor(element.rotation) % 2 === 1;
  const wiper = terminals(element).wiper;
  const flipped = wiper !== undefined && (vertical ? wiper.x > element.x : wiper.y > element.y);
  let sides: Side[] = vertical ? ['right', 'left', 'below', 'above'] : ['below', 'above', 'right', 'left'];
  if (flipped) sides = [sides[1], sides[0], sides[2], sides[3]];
  return { anchor: { x: element.x, y: element.y }, gap: 18, sides };
}

/** A node's voltage sits above and right of its point, clear of the wires that meet there. */
const nodePlace = (point: Point): Place => ({
  anchor: point,
  gap: 8,
  corner: true,
  sides: ['above', 'below', 'right', 'left'],
});

/** The ID and the chosen results of a part. */
function PartLabel({
  element,
  result,
  shown,
  style,
  measure,
}: {
  element: EditorElement;
  result: ElementResult | undefined;
  shown: Shown;
  style: CSSProperties;
  measure: Ref<HTMLDivElement>;
}) {
  return (
    <div className="part-label" style={style} ref={measure} data-label={element.id}>
      <span className="part-id">{element.id}</span>
      {result &&
        QUANTITIES.map(({ key, symbol }) => {
          const value = result[key];
          return shown[key] && value ? <Value key={key} label={symbol} value={value} /> : null;
        })}
    </div>
  );
}

/** A computed value on the canvas: set in KaTeX when known, amber when relative, greyed out when not. */
function Value({ label, value }: { label: string; value: CircuitValue }) {
  const shown = value.status !== 'unknown' && value.result !== null;
  return (
    <div className="canvas-value" title={value.text ?? undefined}>
      <span className="value-label">{label} =</span>{' '}
      {shown ? (
        <Katex latex={value.result!} className={value.status === 'relative' ? 'value-relative' : undefined} />
      ) : (
        <span className="value-unknown">?</span>
      )}
    </div>
  );
}

function Inspector({
  element,
  fields,
  result,
  onChange,
  onRemove,
  onClose,
}: {
  element: EditorElement;
  fields: Field[];
  result: ElementResult | undefined;
  onChange(change: Partial<EditorElement>): void;
  onRemove(): void;
  onClose(): void;
}) {
  const { t, language } = useT();
  const parameters = element as unknown as Record<string, string | undefined>;
  return (
    <div className="inspector">
      <InspectorHead id={element.id} kind={t(KIND_LABELS[element.kind])} onRemove={onRemove} onClose={onClose} />

      {/* A mode with ideal wires only still offers the material when one is set, to take it off. */}
      {element.kind === 'wire' && (fields.length > 0 || element.material) && (
        <label className="field">
          <span>{t('field.material')}</span>
          <select
            value={element.material ?? ''}
            onChange={(event) => onChange({ material: event.target.value || undefined })}
          >
            <option value="">{t('circuit.ideal')}</option>
            {MATERIALS.map((material) => (
              <option key={material.id} value={material.id}>
                {t(material.label)}
              </option>
            ))}
          </select>
        </label>
      )}

      {fields.map((field) => (
        <label className="field" key={field.key}>
          <span>{t(field.label)}</span>
          <input
            type="text"
            spellCheck={false}
            autoComplete="off"
            value={parameters[field.key] ?? ''}
            placeholder={field.placeholder.replace('{,}', language === 'de' ? '{,}' : '.')}
            onChange={(event) => onChange({ [field.key]: event.target.value })}
          />
        </label>
      ))}

      {result && (
        <div className="inspector-results">
          <ResultRow label={t('result.voltage')} value={result.voltage} />
          <ResultRow label={t('result.current')} value={result.current} />
          <ResultRow label={t('result.power')} value={result.power} />
          {result.resistance && <ResultRow label={t('result.resistance')} value={result.resistance} />}
          {result.notes.map((note) => (
            <div className="inspector-note" key={note}>
              {note}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function ResultRow({ label, value }: { label: string; value: CircuitValue }) {
  const { t } = useT();
  return (
    <div className="result-line">
      <span className="result-line-label">{label}</span>
      {value.status === 'known' && value.result !== null ? (
        <Katex latex={value.result} />
      ) : (
        <span className={value.status === 'relative' ? 'value-relative' : 'value-unknown'}>
          {value.text ?? t('result.unknown')}
        </span>
      )}
    </div>
  );
}
