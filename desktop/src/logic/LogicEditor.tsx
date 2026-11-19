import { useEffect, useMemo, useRef, useState, type ReactNode } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import type { GateType, LogicError, LogicSymbols } from '../api';
import { useT } from '../i18n';
import { nameLatex, readName } from '../logic';
import {
  allPins,
  bodyHalf,
  gateName,
  GATES,
  inputCount,
  inverted,
  LOGIC_PALETTE,
  localBody,
  numberNets,
  MAX_GATE_INPUTS,
  newPart,
  PART_LABELS,
  partPins,
  pinOffsets,
  single,
  withGate,
  type LogicPart,
  type PaletteKind,
} from './circuit';
import { labelStyle, nearest, placeLabels, useLabelSizes, type LabelRequest, type Placement, type Side } from '../schematic/labels';
import { boundsOf, connectedPointsOf, freeSpot, GRID, junctionDotsOf, pointKey, segmentBox, sidesAround, turnedBox, wireBetween, wireEnds, type Point, type Rect, type Segment } from '../schematic/geometry';
import { pngBytes, svgBytes, type Picture } from '../schematic/picture';
import { saveBytes } from '../shell';
import { Katex } from '../components/Katex';
import { useSchematicEditing } from '../schematic/editing';
import { InspectorHead, InspectorPopover } from '../schematic/inspector';
import { DragGhost, WirePart } from '../schematic/parts';
import { GridBackground, HOME, useCanvasView, viewTransform, ZoomControls, type Fullscreen } from '../schematic/view';

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
 * the inspector.
 */
export function LogicEditor({ parts, onChange, symbols, toolbar, emptyCanvas, errors = [], warnings = [], values, switchValue, onToggle, corner, note, path = [], fitKey = 0, fullscreen }: Props) {
  const { t } = useT();
  const canvas = useRef<SVGSVGElement>(null);
  const canvasView = useCanvasView(canvas);
  const { view, toGrid } = canvasView;
  const editing = useSchematicEditing({
    elements: parts,
    onChange,
    canvas,
    view: canvasView,
    pins: allPins,
    newWire: (next, from, to) => wireBetween(newPart(next, 'wire', from).id, from, to),
    onTap: (part) => part.kind === 'input' && part.name && onToggle?.(part.name),
    // A click flips a switch; a double click opens its inspector.
    tapOnly: (part) => part.kind === 'input',
  });
  const { selection, inspected, drag, update, remove } = editing;

  const wires = useMemo<Segment[]>(() => parts.filter((p) => p.kind === 'wire').map(wireEnds), [parts]);
  const ends = useMemo(() => parts.flatMap(allPins), [parts]);
  const connected = useMemo(() => connectedPointsOf(ends, wires), [ends, wires]);
  const dots = useMemo(() => junctionDotsOf(ends, wires), [ends, wires]);
  // The value on a point's net: what the wires and lamps show.
  const nets = useMemo(() => numberNets(parts), [parts]);
  const valueAt = (point: Point): boolean | null => {
    const net = nets.byPoint.get(pointKey(point));
    return net === undefined ? null : (values?.get(net) ?? null);
  };
  const valueClass = (value: boolean | null) => (value === null ? undefined : value ? 'on' : 'off');
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
  const wireClass = (a: Point) => {
    const net = nets.byPoint.get(pointKey(a));
    return [valueClass(valueAt(a)), net !== undefined && pathNets.has(net) ? 'critical' : undefined].filter(Boolean).join(' ') || undefined;
  };

  // The names of switches and lamps, and the ids of the gates, laid out clear of the drawing.
  const { sizes, measure } = useLabelSizes();
  const labels = useMemo(() => labelsOf(parts), [parts]);
  const dragging = drag?.type === 'move' || drag?.type === 'end';
  const lastPlacements = useRef<Map<string, Placement>>(new Map());
  const placements = useMemo(() => {
    const requests: LabelRequest[] = [];
    for (const label of labels) {
      const size = sizes.get(label.id);
      if (size) requests.push({ ...label, ...size });
    }
    return placeLabels(requests, parts.flatMap(obstacles), dragging ? lastPlacements.current : undefined);
  }, [labels, sizes, parts, dragging]);
  useEffect(() => {
    lastPlacements.current = placements;
  }, [placements]);

  /** Shows the whole drawing with its labels; the start view without one. */
  const fitAll = () => (parts.length ? canvasView.fit(boundsOf([...parts.flatMap(obstacles), ...placements.values()])) : canvasView.setView(HOME));
  // On opening, and whenever asked, as after a circuit was drawn: once the canvas has its size
  // and every label has been measured, so that nothing is cut off at the edge.
  const pendingFit = useRef(true);
  const [fitTry, setFitTry] = useState(0);
  useEffect(() => {
    pendingFit.current = true;
  }, [fitKey]);
  useEffect(() => {
    if (!pendingFit.current) return;
    const width = canvas.current?.getBoundingClientRect().width ?? 0;
    const measured = labels.every((label) => sizes.has(label.id));
    if (!width || !measured) {
      // Not laid out yet: look again in the next frame, a few times at most.
      if (fitTry < 30) {
        const frame = requestAnimationFrame(() => setFitTry((n) => n + 1));
        return () => cancelAnimationFrame(frame);
      }
    }
    pendingFit.current = false;
    setFitTry(0);
    fitAll();
    // Only when asked, not on every change of the drawing.
  }, [fitKey, placements, fitTry]);

  const add = (kind: PaletteKind, at?: Point) => {
    // Without a drop position the part goes to a free spot, one row below the last one.
    const spot = at ?? freeSpot(view, parts.length, 5);
    const part = newPart(parts, kind, spot);
    onChange([...parts, part]);
    editing.inspect(part.id);
    // The canvas takes the keys, so that Del removes the new part at once.
    canvas.current?.focus();
  };

  const onKeyDown = (event: React.KeyboardEvent) => editing.keyDown(event, { home: fitAll, fit: fitAll });

  const faulty = useMemo(() => new Set(errors.flatMap((e) => e.elements)), [errors]);
  const doubtful = useMemo(() => new Set(warnings.flatMap((e) => e.elements)), [warnings]);
  const mark = (id: string) => (faulty.has(id) ? 'error' : doubtful.has(id) ? 'warning' : null);
  const notes = errors.length ? errors : warnings;
  const noteShown = notes.length > 0 || note;

  /** A switch's value, or the value a lamp shows; null when not known. */
  const simulated = (part: LogicPart): boolean | null => {
    if (part.kind === 'input') return part.name ? (switchValue?.(part.name) ?? null) : null;
    if (part.kind === 'output') return valueAt(partPins(part).inputs[0]);
    return null;
  };

  const chosen = (drag?.type !== 'move' && parts.find((p) => p.id === inspected)) || null;

  /** The circuit as a picture file (§10.5), the SVG or a PNG of it. */
  const [exportError, setExportError] = useState<string | null>(null);
  const exportPicture = async (format: 'svg' | 'png') => {
    try {
      const picture = circuitPicture(parts, symbols, labels, placements, dots);
      const bytes = format === 'svg' ? svgBytes(picture) : await pngBytes(picture);
      await saveBytes(bytes, `${t('logic.export.name')}.${format}`, { name: format.toUpperCase(), extensions: [format] });
      setExportError(null);
    } catch (error) {
      setExportError(error instanceof Error ? error.message : String(error));
    }
  };

  return (
    <div className="circuit logic-editor">
      <div className="circuit-palette">
        <div className="palette-title">{t('circuit.parts')}</div>
        {LOGIC_PALETTE.map((kind) => (
          <button
            key={kind}
            className="palette-item"
            draggable
            title={t('circuit.drag')}
            onDragStart={(event) => event.dataTransfer.setData('text/plain', kind)}
            onClick={() => add(kind)}
          >
            <svg viewBox="-46 -34 92 68" width="40" height="30">
              <PartSymbol part={paletteSample(kind)} symbols={symbols} />
            </svg>
            <span>{paletteLabel(kind, t)}</span>
          </button>
        ))}
        <div className="logic-export">
          <div className="palette-title">{t('logic.export.title')}</div>
          <div className="logic-export-buttons">
            <button className="form-chip" title={t('logic.export.svg.hint')} disabled={!parts.length} onClick={() => void exportPicture('svg')}>
              SVG
            </button>
            <button className="form-chip" title={t('logic.export.png.hint')} disabled={!parts.length} onClick={() => void exportPicture('png')}>
              PNG
            </button>
          </div>
          {exportError && <p className="file-error">{exportError}</p>}
        </div>
        <p className="palette-hint">
          {t('logic.editor.hint')} <kbd>{t('key.space')}</kbd> {t('circuit.rotates')}, <kbd>{t('key.delete')}</kbd> {t('circuit.deletes')}.
        </p>
        <p className="palette-hint">
          {t('circuit.pan')} <kbd>{t('key.home')}</kbd> {t('logic.editor.home')}.
        </p>
      </div>

      <div className="circuit-centre">
        {toolbar && <div className="logic-toolbar">{toolbar}</div>}
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
              const kind = event.dataTransfer.getData('text/plain') as PaletteKind;
              if (LOGIC_PALETTE.includes(kind)) add(kind, toGrid(event));
            }}
          >
            <GridBackground view={view} />
            <g transform={viewTransform(view)}>
              {parts.map((part) => {
                const className = ['part', selection.includes(part.id) && 'selected', mark(part.id), onPath.has(part.id) && 'critical'].filter(Boolean).join(' ');
                if (part.kind === 'wire') {
                  const [a, b] = wireEnds(part);
                  return (
                    <WirePart
                      key={part.id}
                      a={a}
                      b={b}
                      className={className}
                      lineClassName={wireClass(a)}
                      joins
                      connected={connected}
                      onPointerDown={(event) => editing.startMove(event, part)}
                      onEnd={(event, end) => editing.startEnd(event, part, end)}
                      onTerminal={(event, point) => editing.startDraw(event, point, part)}
                    />
                  );
                }
                return (
                  <g key={part.id} className={className}>
                    <g transform={`translate(${part.x} ${part.y}) rotate(${part.rotation * 90})`}>
                      <g className="symbol">
                        <PartSymbol part={part} symbols={symbols} value={simulated(part)} />
                      </g>
                      <HitArea part={part} onPointerDown={(event) => editing.startMove(event, part)} />
                    </g>
                    {allPins(part).map((point, k) => (
                      <circle
                        key={k}
                        className={connected.has(pointKey(point)) ? 'terminal connected' : 'terminal'}
                        cx={point.x}
                        cy={point.y}
                        r="4"
                        onPointerDown={(event) => editing.startDraw(event, point)}
                      />
                    ))}
                  </g>
                );
              })}

              {dots.map((point) => (
                <circle key={pointKey(point)} className="junction-dot" cx={point.x} cy={point.y} r="3.5" />
              ))}

              {labels.map((label) => {
                const placement = placements.get(label.id);
                if (!placement?.displaced) return null;
                const centre = { x: placement.x + placement.w / 2, y: placement.y + placement.h / 2 };
                const from = nearest(bodyBox(label.part), centre);
                const to = nearest(placement, from);
                return <line key={label.id} className="label-leader" x1={from.x} y1={from.y} x2={to.x} y2={to.y} />;
              })}

              <DragGhost drag={drag} />
            </g>
          </svg>

          <div className="circuit-labels">
            <div className="circuit-labels-drawing" style={{ transform: `translate(${view.x}px, ${view.y}px) scale(${view.zoom})` }}>
              {labels.map((label) => (
                <div
                  key={label.id}
                  className={label.part.kind === 'gate' ? 'logic-gate-label' : 'logic-name-label'}
                  style={labelStyle(placements.get(label.id), label.anchor)}
                  ref={measure}
                  data-label={label.id}
                >
                  {label.part.kind === 'gate' ? label.part.id : <Katex latex={nameLatex(label.part.name ?? '')} />}
                </div>
              ))}
            </div>
          </div>

          {!parts.length && emptyCanvas && <div className="logic-canvas-empty">{emptyCanvas}</div>}

          {chosen && (
            <InspectorPopover box={chosen.kind === 'wire' ? segmentBox(...wireEnds(chosen), 2) : bodyBox(chosen)} view={view} canvas={canvas}>
              <Inspector
                part={chosen}
                parts={parts}
                onChange={(change) => update(chosen.id, change)}
                onReplace={(next) => onChange(parts.map((p) => (p.id === next.id ? next : p)))}
                onRemove={() => remove([chosen.id])}
                onClose={editing.closeInspector}
              />
            </InspectorPopover>
          )}

        </div>

        {(corner || noteShown) && (
          <div className="logic-editor-bar">
            {corner}
            {noteShown && (
              <div className={errors.length ? 'logic-bar-note error' : 'logic-bar-note'}>
                {notes.map((issue, k) => (
                  <div key={k}>{issue.message}</div>
                ))}
                {!errors.length && note}
              </div>
            )}
          </div>
        )}
      </div>

    </div>
  );
}

/** The colours of a picture: ink, the paper (also the fill of bodies), the knob's and the faint labels' grey. */
interface PictureColors {
  ink: string;
  paper: string;
  muted: string;
  faint: string;
}

/** An exported picture: the light theme's colours, black on white. */
const EXPORT_COLORS: PictureColors = { ink: '#1b1a22', paper: '#ffffff', muted: '#6b6878', faint: '#8a8698' };

/** The app's colours as the current theme sets them, for a picture shown in the app. */
function appColors(): PictureColors {
  const style = getComputedStyle(document.documentElement);
  const read = (name: string, fallback: string) => style.getPropertyValue(name).trim() || fallback;
  return { ink: read('--text', EXPORT_COLORS.ink), paper: read('--background', EXPORT_COLORS.paper), muted: read('--text-muted', EXPORT_COLORS.muted), faint: read('--text-faint', EXPORT_COLORS.faint) };
}

const pictureStyle = ({ ink, paper, muted, faint }: PictureColors) => `
.symbol, .wire { fill: none; stroke: ${ink}; stroke-width: 2; stroke-linecap: round; stroke-linejoin: round; }
.symbol .logic-body, .symbol .logic-bubble, .symbol .logic-lamp, .symbol .logic-switch { fill: ${paper}; }
.symbol .logic-knob { fill: ${muted}; stroke: none; }
.logic-sign { fill: ${ink}; stroke: none; font-family: ui-monospace, Menlo, Consolas, monospace; font-size: 11px; text-anchor: middle; dominant-baseline: middle; }
.junction-dot { fill: ${ink}; }
.label-leader { stroke: ${faint}; stroke-width: 1; stroke-dasharray: 2 3; }
.name { fill: ${ink}; font-family: 'Latin Modern Roman', 'Times New Roman', serif; font-size: 15px; text-anchor: middle; dominant-baseline: central; }
.name .letter { font-style: italic; }
.id { fill: ${faint}; font-family: ui-monospace, Menlo, Consolas, monospace; font-size: 10px; text-anchor: middle; dominant-baseline: central; }
`;

/** The labels of the parts that have one: a switch's or lamp's name, a gate's id. */
const labelsOf = (parts: LogicPart[]): (LabelOf & Omit<LabelRequest, 'w' | 'h'>)[] =>
  parts.flatMap((part) => {
    const place = labelPlace(part);
    return place ? [{ ...place, id: part.id, part }] : [];
  });

/**
 * A circuit as a picture without an editor (the Gates card): the labels' sizes guessed from their
 * text instead of measured, in the app's colours (`theme: 'app'`) or black on white.
 */
export function staticPicture(parts: LogicPart[], symbols: LogicSymbols, theme: 'app' | 'export'): Picture {
  const labels = labelsOf(parts);
  const size = (part: LogicPart) => {
    if (part.kind === 'gate') return { w: 7 * part.id.length + 6, h: 12 };
    const [base, index = ''] = (part.name ?? '').split(/_(.*)/s);
    return { w: 9 * base.length + 6 * index.length + 6, h: 18 };
  };
  const requests = labels.map((label) => ({ ...label, ...size(label.part) }));
  const placements = placeLabels(requests, parts.flatMap(obstacles));
  const ends = parts.flatMap(allPins);
  const wires = parts.filter((p) => p.kind === 'wire').map(wireEnds);
  return circuitPicture(parts, symbols, labels, placements, junctionDotsOf(ends, wires), theme === 'app' ? appColors() : EXPORT_COLORS, theme === 'export');
}

type LabelOf = { id: string; anchor: Point; part: LogicPart };

/** A switch's or lamp's name as SVG text: a letter in italics, a longer name upright, the index lowered. */
function NameText({ name, x, y }: { name: string; x: number; y: number }) {
  const [base, index] = name.split(/_(.*)/s);
  return (
    <text className="name" x={x} y={y}>
      <tspan className={base.length === 1 ? 'letter' : undefined}>{base}</tspan>
      {index !== undefined && (
        <tspan dy="4" fontSize="10">
          {index}
        </tspan>
      )}
    </text>
  );
}

/**
 * The circuit as a standalone SVG (§10.5): the parts, wires, junctions and labels where the
 * editor shows them, without the grid, the pins, a selection or the simulation, in the light
 * theme's colours on white.
 */
function circuitPicture(parts: LogicPart[], symbols: LogicSymbols, labels: LabelOf[], placements: Map<string, Placement>, dots: Point[], colors = EXPORT_COLORS, paper = true): Picture {
  const margin = 16;
  const boxOf = (label: LabelOf): Rect => placements.get(label.id) ?? { x: label.anchor.x, y: label.anchor.y, w: 0, h: 0 };
  const box = boundsOf([...parts.flatMap(obstacles), ...labels.map(boxOf)]);
  const [x, y, width, height] = [box.x - margin, box.y - margin, Math.ceil(box.w + 2 * margin), Math.ceil(box.h + 2 * margin)];
  const body = renderToStaticMarkup(
    <>
      {parts.map((part) => {
        if (part.kind === 'wire') {
          const [a, b] = wireEnds(part);
          return <line key={part.id} className="wire" x1={a.x} y1={a.y} x2={b.x} y2={b.y} />;
        }
        return (
          <g key={part.id} className="symbol" transform={`translate(${part.x} ${part.y}) rotate(${part.rotation * 90})`}>
            <PartSymbol part={part} symbols={symbols} />
          </g>
        );
      })}
      {dots.map((point) => (
        <circle key={pointKey(point)} className="junction-dot" cx={point.x} cy={point.y} r="3.5" />
      ))}
      {labels.map((label) => {
        const placement = placements.get(label.id);
        const at = boxOf(label);
        const centre = { x: at.x + at.w / 2, y: at.y + at.h / 2 };
        const leader = placement?.displaced && (() => {
          const from = nearest(bodyBox(label.part), centre);
          const to = nearest(placement, from);
          return <line className="label-leader" x1={from.x} y1={from.y} x2={to.x} y2={to.y} />;
        })();
        return (
          <g key={label.id}>
            {leader}
            {label.part.kind === 'gate' ? (
              <text className="id" x={centre.x} y={centre.y}>
                {label.part.id}
              </text>
            ) : (
              <NameText name={label.part.name ?? ''} x={centre.x} y={centre.y} />
            )}
          </g>
        );
      })}
    </>,
  );
  const svg =
    `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="${x} ${y} ${width} ${height}">` +
    `<style>${pictureStyle(colors)}</style>${paper ? `<rect x="${x}" y="${y}" width="${width}" height="${height}" fill="${colors.paper}"/>` : ''}${body}</svg>`;
  return { svg, width, height };
}

/** A part of a palette kind as the palette shows it. */
function paletteSample(kind: PaletteKind): LogicPart {
  return newPart([], kind, { x: 0, y: 0 });
}

function paletteLabel(kind: PaletteKind, t: ReturnType<typeof useT>['t']): string {
  return kind === 'input' || kind === 'output' || kind === 'constant' || kind === 'wire' ? t(PART_LABELS[kind]) : gateName(kind);
}

/** Where a part's label would like to sit: a switch's name on its left, a lamp's on its right, a gate's id above it. */
function labelPlace(part: LogicPart): Pick<LabelRequest, 'anchor' | 'sides' | 'gap'> | null {
  const box = bodyBox(part);
  const centre = { x: box.x + box.w / 2, y: box.y + box.h / 2 };
  const vertical = part.rotation % 2 === 1;
  if (part.kind === 'gate') {
    return { anchor: centre, sides: vertical ? sidesAround('right', 'above') : sidesAround('above', 'right'), gap: (vertical ? box.w : box.h) / 2 + 2 };
  }
  if (part.kind !== 'input' && part.kind !== 'output') return null;
  // The side away from the pin: a switch's pin points right when unturned, a lamp's left.
  const away: Side[] = ['left', 'above', 'right', 'below'];
  const turn = (part.rotation + (part.kind === 'output' ? 2 : 0)) % 4;
  const first = away[turn];
  // Across, the side of the pin comes last.
  const sides = first === 'left' || first === 'right' ? sidesAround(first, 'above', true) : sidesAround(first, 'left');
  return { anchor: centre, sides, gap: (first === 'left' || first === 'right' ? box.w : box.h) / 2 + 4 };
}

/** The box from (x0, y0) to (x1, y1) of a part's own coordinates, turned with it and moved to its place. */
const boxOfLocal = (part: LogicPart, x0: number, y0: number, x1: number, y1: number): Rect => turnedBox(part, [{ x: x0, y: y0 }, { x: x1, y: y1 }]);

/** A part's drawn body on the canvas. */
const bodyBox = (part: LogicPart): Rect => boxOfLocal(part, ...localBody(part));

/** What the labels keep clear of: a part's body, leads and pins; a wire's line. */
function obstacles(part: LogicPart): Rect[] {
  if (part.kind === 'wire') {
    const [a, b] = wireEnds(part);
    return [segmentBox(a, b, 2)];
  }
  const [left, , right] = localBody(part);
  // The leads run across from the pins to the body, in the part's own coordinates.
  const offsets = part.kind === 'gate' ? pinOffsets(inputCount(part)).map((o) => o * GRID) : [];
  const leads = [
    ...offsets.map((y) => boxOfLocal(part, -2 * GRID, y - 2, left, y + 2)),
    ...(part.kind === 'gate' ? [boxOfLocal(part, right, -2, 2 * GRID, 2)] : []),
    ...(part.kind === 'input' || part.kind === 'constant' ? [boxOfLocal(part, right, -2, GRID, 2)] : []),
    ...(part.kind === 'output' ? [boxOfLocal(part, -GRID, -2, left, 2)] : []),
  ];
  return [bodyBox(part), ...leads, ...allPins(part).map((pin) => segmentBox(pin, pin, 5))];
}

/** Where a part is grabbed: its body with a little room. */
function HitArea({ part, onPointerDown }: { part: LogicPart; onPointerDown(event: React.PointerEvent): void }) {
  const [x0, y0, x1, y1] = localBody(part);
  return <rect className="part-hit" x={x0 - 4} y={y0 - 4} width={x1 - x0 + 8} height={y1 - y0 + 8} onPointerDown={onPointerDown} />;
}

/** The sign in an IEC gate's box. */
const IEC_SIGN: Record<GateType, string> = { and: '&', nand: '&', or: '≥1', nor: '≥1', xor: '=1', xnor: '=1', not: '1', buffer: '1' };

/** The x of the back of an ANSI OR at height `y` of its half height `h`: a curve from (`back`, −h) over (`back` + 12, 0) to (`back`, h). */
const orBack = (back: number, h: number, y: number) => {
  const t = (y / h + 1) / 2;
  return back + 24 * t * (1 - t);
};

/**
 * A part in its own coordinates, unturned: a gate's inputs at (−2 grid, row), its output at (2 grid,
 * 0); a switch's and a constant's pin at (1 grid, 0), a lamp's at (−1 grid, 0).
 */
function PartSymbol({ part, symbols, value = null }: { part: LogicPart; symbols: LogicSymbols; value?: boolean | null }) {
  const state = value === null ? '' : value ? ' on' : ' off';
  switch (part.kind) {
    case 'wire':
      return <path d={`M${-2 * GRID} 0H${2 * GRID}`} />;
    case 'input':
      return (
        <>
          <path d={`M8 0H${GRID}`} />
          <rect className={`logic-switch${state}`} x="-16" y="-9" width="24" height="18" rx="9" />
          <circle className={`logic-knob${state}`} cx={value ? -1 : -7} cy="0" r="5" />
        </>
      );
    case 'constant':
      return (
        <>
          <path d={`M8 0H${GRID}`} />
          <rect x="-12" y="-9" width="20" height="18" rx="2" />
          <text className="logic-sign" x="-2" y="0">
            {part.value ? '1' : '0'}
          </text>
        </>
      );
    case 'output':
      return (
        <>
          <path d={`M${-GRID} 0H-9`} />
          <circle className={`logic-lamp${state}`} r="9" />
          <path d="M-6.4 -6.4L6.4 6.4M-6.4 6.4L6.4 -6.4" />
        </>
      );
    case 'gate':
      return <GateSymbol part={part} symbols={symbols} />;
  }
}

function GateSymbol({ part, symbols }: { part: LogicPart; symbols: LogicSymbols }) {
  const gate = part.gate ?? 'and';
  const h = bodyHalf(part);
  const offsets = pinOffsets(inputCount(part));
  const bubble = inverted(gate);
  const right = GRID;
  const out = <path d={`M${bubble ? right + 8 : right} 0H${2 * GRID}`} />;
  const mark = bubble && <circle className="logic-bubble" cx={right + 4} cy="0" r="4" />;
  if (symbols === 'iec') {
    const half = single(gate) ? 16 : h;
    return (
      <>
        {offsets.map((o) => (
          <path key={o} d={`M${-2 * GRID} ${o * GRID}H${-GRID}`} />
        ))}
        {out}
        <rect className="logic-body" x={-GRID} y={-half} width={2 * GRID} height={2 * half} />
        {mark}
        <text className="logic-sign" x="0" y={-half + 11} transform={`rotate(${-part.rotation * 90} 0 ${-half + 11})`}>
          {IEC_SIGN[gate]}
        </text>
      </>
    );
  }
  // ANSI: the distinctive shapes, from x = −grid to x = +grid.
  const leads = (to: (y: number) => number) =>
    offsets.map((o) => <path key={o} d={`M${-2 * GRID} ${o * GRID}H${to(o * GRID)}`} />);
  switch (gate) {
    case 'not':
    case 'buffer':
      return (
        <>
          {leads(() => -16)}
          {out}
          <path className="logic-body" d={`M-16 -14L${right} 0L-16 14Z`} />
          {mark}
        </>
      );
    case 'and':
    case 'nand':
      return (
        <>
          {leads(() => -GRID)}
          {out}
          <path className="logic-body" d={`M${-GRID} ${-h}H0A${GRID} ${h} 0 0 1 0 ${h}H${-GRID}Z`} />
          {mark}
        </>
      );
    case 'or':
    case 'nor':
      return (
        <>
          {leads((y) => orBack(-GRID, h, y))}
          {out}
          <path className="logic-body" d={`M${-GRID} ${-h}Q4 ${-h} ${GRID} 0Q4 ${h} ${-GRID} ${h}Q-8 0 ${-GRID} ${-h}Z`} />
          {mark}
        </>
      );
    case 'xor':
    case 'xnor':
      return (
        <>
          {leads((y) => orBack(-GRID, h, y))}
          {out}
          <path className="logic-body" d={`M-14 ${-h}Q6 ${-h} ${GRID} 0Q6 ${h} -14 ${h}Q-2 0 -14 ${-h}Z`} />
          <path d={`M${-GRID} ${-h}Q-8 0 ${-GRID} ${h}`} />
          {mark}
        </>
      );
  }
}

/** A part's settings: a switch's or lamp's name, a constant's value, a gate's type and number of inputs. */
function Inspector({
  part,
  parts,
  onChange,
  onReplace,
  onRemove,
  onClose,
}: {
  part: LogicPart;
  parts: LogicPart[];
  onChange(change: Partial<LogicPart>): void;
  onReplace(part: LogicPart): void;
  onRemove(): void;
  onClose(): void;
}) {
  const { t } = useT();
  const kind = part.kind === 'gate' ? gateName(part.gate ?? 'and') : t(PART_LABELS[part.kind]);
  return (
    <div className="inspector">
      <InspectorHead id={part.id} kind={kind} onRemove={onRemove} onClose={onClose} />

      {(part.kind === 'input' || part.kind === 'output') && (
        <NameField
          key={part.id}
          name={part.name ?? ''}
          taken={parts.filter((p) => p.kind === part.kind && p.id !== part.id).map((p) => p.name ?? '')}
          onRename={(name) => onChange({ name })}
        />
      )}

      {part.kind === 'constant' && (
        <label className="field">
          <span>{t('logic.field.value')}</span>
          <select value={part.value ? '1' : '0'} onChange={(event) => onChange({ value: event.target.value === '1' })}>
            <option value="0">0</option>
            <option value="1">1</option>
          </select>
        </label>
      )}

      {part.kind === 'gate' && (
        <>
          <label className="field">
            <span>{t('logic.field.gate')}</span>
            <select value={part.gate} onChange={(event) => onReplace(withGate(part, event.target.value as GateType))}>
              {GATES.map((gate) => (
                <option key={gate} value={gate}>
                  {gateName(gate)}
                </option>
              ))}
            </select>
          </label>
          {!single(part.gate) && (
            <label className="field">
              <span>{t('logic.field.inputs')}</span>
              <select value={inputCount(part)} onChange={(event) => onChange({ inputs: Number(event.target.value) })}>
                {Array.from({ length: MAX_GATE_INPUTS - 1 }, (_, k) => k + 2).map((n) => (
                  <option key={n} value={n}>
                    {n}
                  </option>
                ))}
              </select>
            </label>
          )}
        </>
      )}
    </div>
  );
}

/** A switch's or lamp's name, taken when it is a name no other of its kind has. */
function NameField({ name, taken, onRename }: { name: string; taken: string[]; onRename(name: string): void }) {
  const { t } = useT();
  const [text, setText] = useState(name);
  useEffect(() => setText(name), [name]);
  const read = readName(text);
  const valid = read !== null && !taken.includes(read);
  return (
    <label className="field">
      <span>{t('logic.field.name')}</span>
      <input
        type="text"
        spellCheck={false}
        autoComplete="off"
        className={valid ? undefined : 'invalid'}
        title={valid ? undefined : t('logic.table.badName')}
        value={text}
        onChange={(event) => {
          setText(event.target.value);
          const next = readName(event.target.value);
          if (next !== null && !taken.includes(next)) onRename(next);
        }}
      />
    </label>
  );
}
