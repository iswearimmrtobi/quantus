import { useEffect, useMemo, useRef, useState, type PointerEvent as ReactPointerEvent, type ReactNode } from 'react';
import { api, ApiError, type CircuitResponse, type CircuitValue, type Settings } from '../api';
import {
  FIELDS,
  GRID,
  HALF,
  KIND_LABELS,
  MATERIALS,
  newElement,
  QUANTITIES,
  numberNodes,
  pointKey,
  rotate,
  snap,
  terminals,
  toRequest,
  type EditorElement,
  type EditorKind,
  type Point,
  type Shown,
  type Terminal,
} from '../circuit';
import { useT } from '../i18n';
import { Katex } from './Katex';

interface Props {
  elements: EditorElement[];
  onChange(next: EditorElement[]): void;
  /** The solver reads the stored settings; these are watched so a change shows at once. */
  settings: Settings;
  /** Which results the parts show on the canvas. */
  shown: Shown;
  onShownChange(next: Shown): void;
  /** Extra controls for the palette column (load and save). */
  toolbar?: ReactNode;
}

type Drag =
  | { type: 'move'; from: Point; origins: Map<string, EditorElement> }
  | { type: 'end'; id: string; end: 'a' | 'b' }
  | { type: 'draw'; from: Point; to: Point }
  | { type: 'marquee'; from: Point; to: Point }
  | { type: 'pan'; from: Point; origin: View };

/** The drawing is shown scaled by `zoom` and then moved by (x, y) on the canvas. */
interface View {
  x: number;
  y: number;
  zoom: number;
}

const HOME: View = { x: 0, y: 0, zoom: 1 };
const MIN_ZOOM = 0.25;
const MAX_ZOOM = 4;

const PALETTE: EditorKind[] = [
  'voltageSource',
  'currentSource',
  'resistor',
  'potentiometer',
  'powerLoad',
  'wire',
  'ground',
];

export function CircuitEditor({ elements, onChange, settings, shown, onShownChange, toolbar }: Props) {
  const { t } = useT();
  const [selection, setSelection] = useState<string[]>([]);
  const [drag, setDrag] = useState<Drag | null>(null);
  /** How the drawing lies on the canvas; positions stay in drawing coordinates. */
  const [view, setView] = useState<View>(HOME);
  const [response, setResponse] = useState<CircuitResponse | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const canvas = useRef<SVGSVGElement>(null);

  const nodes = useMemo(() => numberNodes(elements), [elements]);
  const request = useMemo(() => toRequest(elements, nodes), [elements, nodes]);

  // Every change recalculates, and so does a changed language, precision or output setting; the
  // engine runs in the app, so a short debounce is enough to keep dragging smooth. A slow answer
  // to an older circuit must not overwrite a newer one.
  const { language, precision, exactRoots, letterProducts, extraUnitLetters, mode } = settings;
  useEffect(() => {
    let current = true;
    const timer = window.setTimeout(() => {
      if (request.elements.length === 0) {
        setResponse(null);
        setFailure(null);
        return;
      }
      api.solveCircuit(request).then(
        (solved) => {
          if (!current) return;
          setResponse(solved);
          setFailure(null);
        },
        (error) => {
          if (!current) return;
          setResponse(null);
          setFailure(error instanceof ApiError ? error.message : String(error));
        },
      );
    }, 60);
    return () => {
      current = false;
      window.clearTimeout(timer);
    };
  }, [request, language, precision, exactRoots, letterProducts, extraUnitLetters, mode]);

  const results = useMemo(() => new Map(response?.elements.map((e) => [e.id, e])), [response]);
  const voltages = useMemo(() => new Map(response?.nodes.map((n) => [n.id, n.voltage])), [response]);

  const update = (id: string, change: Partial<EditorElement>) =>
    onChange(elements.map((e) => (e.id === id ? { ...e, ...change } : e)));

  const selected = selection.length === 1 ? selection[0] : null;

  const add = (kind: EditorKind, at?: Point) => {
    // Without a drop position the part goes to a free spot, one row below the last one.
    const spot = at ?? {
      x: (160 - view.x) / view.zoom + (elements.length % 4) * 3 * GRID,
      y: (120 - view.y) / view.zoom + Math.floor(elements.length / 4) * 3 * GRID,
    };
    const element = newElement(elements, kind, spot);
    onChange([...elements, element]);
    setSelection([element.id]);
  };

  const remove = (ids: string[]) => {
    onChange(elements.filter((e) => !ids.includes(e.id)));
    setSelection([]);
  };

  const rotateSelected = () => {
    const chosen = elements.filter((e) => selection.includes(e.id));
    if (chosen.length === 0) return;
    // One part turns about itself (a wire about its start); several turn together about the
    // middle of their bounding box, so they stay connected to each other.
    let center: Point;
    if (chosen.length === 1) {
      center = { x: chosen[0].x, y: chosen[0].y };
    } else {
      const points = chosen.flatMap((e) => Object.values(terminals(e)) as Point[]);
      const xs = points.map((p) => p.x);
      const ys = points.map((p) => p.y);
      center = { x: snap((Math.min(...xs) + Math.max(...xs)) / 2), y: snap((Math.min(...ys) + Math.max(...ys)) / 2) };
    }
    const turn = (x: number, y: number): Point => {
      const turned = rotate({ x: x - center.x, y: y - center.y }, 1);
      return { x: center.x + turned.x, y: center.y + turned.y };
    };
    onChange(
      elements.map((e) => {
        if (!selection.includes(e.id)) return e;
        const start = turn(e.x, e.y);
        if (e.kind === 'wire') {
          const end = turn(e.x2 ?? e.x, e.y2 ?? e.y);
          return { ...e, x: start.x, y: start.y, x2: end.x, y2: end.y };
        }
        return { ...e, x: start.x, y: start.y, rotation: (e.rotation + 1) % 4 };
      }),
    );
  };

  /** The point under the pointer in drawing coordinates, unsnapped. */
  const toDrawing = (event: { clientX: number; clientY: number }): Point => {
    const box = canvas.current!.getBoundingClientRect();
    return { x: (event.clientX - box.left - view.x) / view.zoom, y: (event.clientY - box.top - view.y) / view.zoom };
  };

  // A wheel scrolls up and down, with Shift sideways; a touchpad scrolls both ways. With Ctrl (and
  // a touchpad pinch, which arrives as one) it zooms about the pointer. React's wheel handler is
  // passive and could not keep the webview from zooming the whole page, so this one is native.
  useEffect(() => {
    const svg = canvas.current;
    if (!svg) return;
    const onWheel = (event: WheelEvent) => {
      event.preventDefault();
      const unit = event.deltaMode === WheelEvent.DOM_DELTA_LINE ? 16 : 1;
      if (event.ctrlKey) {
        const box = svg.getBoundingClientRect();
        const at = { x: event.clientX - box.left, y: event.clientY - box.top };
        setView((v) => {
          const zoom = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, v.zoom * Math.exp(-event.deltaY * unit * 0.002)));
          // The drawing point under the pointer stays under it.
          return { zoom, x: at.x - ((at.x - v.x) * zoom) / v.zoom, y: at.y - ((at.y - v.y) * zoom) / v.zoom };
        });
        return;
      }
      const sideways = event.shiftKey && event.deltaX === 0;
      const dx = (sideways ? event.deltaY : event.deltaX) * unit;
      const dy = (sideways ? 0 : event.deltaY) * unit;
      setView((v) => ({ ...v, x: v.x - dx, y: v.y - dy }));
    };
    svg.addEventListener('wheel', onWheel, { passive: false });
    return () => svg.removeEventListener('wheel', onWheel);
  }, []);

  const toCanvas = (event: { clientX: number; clientY: number }): Point => {
    const at = toDrawing(event);
    return { x: snap(at.x), y: snap(at.y) };
  };

  // Only the left button works on parts; the others fall through to the canvas and move it.
  const startMove = (event: ReactPointerEvent, element: EditorElement) => {
    if (event.button !== 0) return;
    event.stopPropagation();
    canvas.current?.setPointerCapture(event.pointerId);
    // Grabbing a part of the selection moves all of it; any other part becomes the selection.
    const group = selection.includes(element.id) ? selection : [element.id];
    setSelection(group);
    setDrag({
      type: 'move',
      from: toCanvas(event),
      origins: new Map(elements.filter((e) => group.includes(e.id)).map((e) => [e.id, e])),
    });
  };

  const startEnd = (event: ReactPointerEvent, element: EditorElement, end: 'a' | 'b') => {
    if (event.button !== 0) return;
    event.stopPropagation();
    canvas.current?.setPointerCapture(event.pointerId);
    setSelection([element.id]);
    setDrag({ type: 'end', id: element.id, end });
  };

  const startDraw = (event: ReactPointerEvent, from: Point) => {
    if (event.button !== 0) return;
    event.stopPropagation();
    canvas.current?.setPointerCapture(event.pointerId);
    setDrag({ type: 'draw', from, to: from });
  };

  /** Left on empty canvas draws a selection rectangle; the middle or right button moves the canvas. */
  const startCanvas = (event: ReactPointerEvent) => {
    canvas.current?.setPointerCapture(event.pointerId);
    canvas.current?.focus();
    if (event.button === 1 || event.button === 2) {
      event.preventDefault();
      setDrag({ type: 'pan', from: { x: event.clientX, y: event.clientY }, origin: view });
      return;
    }
    const from = toDrawing(event);
    setSelection([]);
    setDrag({ type: 'marquee', from, to: from });
  };

  const onMove = (event: ReactPointerEvent) => {
    if (!drag) return;
    if (drag.type === 'pan') {
      setView({ ...drag.origin, x: drag.origin.x + event.clientX - drag.from.x, y: drag.origin.y + event.clientY - drag.from.y });
      return;
    }
    if (drag.type === 'marquee') {
      setDrag({ ...drag, to: toDrawing(event) });
      return;
    }
    const at = toCanvas(event);
    if (drag.type === 'draw') {
      if (at.x !== drag.to.x || at.y !== drag.to.y) setDrag({ ...drag, to: at });
    } else if (drag.type === 'end') {
      update(drag.id, drag.end === 'a' ? { x: at.x, y: at.y } : { x2: at.x, y2: at.y });
    } else {
      const dx = at.x - drag.from.x;
      const dy = at.y - drag.from.y;
      onChange(
        elements.map((e) => {
          const o = drag.origins.get(e.id);
          if (!o) return e;
          return o.kind === 'wire'
            ? { ...e, x: o.x + dx, y: o.y + dy, x2: (o.x2 ?? o.x) + dx, y2: (o.y2 ?? o.y) + dy }
            : { ...e, x: o.x + dx, y: o.y + dy };
        }),
      );
    }
  };

  const onUp = () => {
    if (drag?.type === 'draw' && pointKey(drag.from) !== pointKey(drag.to)) {
      const wire = { ...newElement(elements, 'wire', drag.from), x2: drag.to.x, y2: drag.to.y };
      onChange([...elements, wire]);
      setSelection([wire.id]);
    } else if (drag?.type === 'marquee') {
      const [x0, x1] = [Math.min(drag.from.x, drag.to.x), Math.max(drag.from.x, drag.to.x)];
      const [y0, y1] = [Math.min(drag.from.y, drag.to.y), Math.max(drag.from.y, drag.to.y)];
      const inside = (p: Point) => p.x >= x0 && p.x <= x1 && p.y >= y0 && p.y <= y1;
      // A part is picked when its position or any of its terminals lies in the rectangle.
      setSelection(
        elements
          .filter((e) => [{ x: e.x, y: e.y }, ...(Object.values(terminals(e)) as Point[])].some(inside))
          .map((e) => e.id),
      );
    }
    setDrag(null);
  };

  const onKeyDown = (event: React.KeyboardEvent) => {
    if (event.key === 'Delete' || event.key === 'Backspace') {
      remove(selection);
    } else if (event.key === ' ' || event.key.toLowerCase() === 'r') {
      rotateSelected();
    } else if (event.key === 'Escape') {
      setSelection([]);
    } else if (event.key === 'Home') {
      setView(HOME);
    } else {
      return;
    }
    event.preventDefault();
  };

  const chosen = elements.find((e) => e.id === selected) ?? null;

  return (
    <div className="circuit">
      <div className="circuit-palette">
        <div className="palette-title">{t('circuit.parts')}</div>
        {PALETTE.map((kind) => (
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
          {t('circuit.hint')} <kbd>{t('key.space')}</kbd> {t('circuit.rotates')}, <kbd>{t('key.delete')}</kbd> {t('circuit.deletes')}.
        </p>
        <p className="palette-hint">
          {t('circuit.pan')} <kbd>{t('key.home')}</kbd> {t('circuit.home')}.
        </p>
      </div>

      <div className="circuit-stage">
        <svg
          ref={canvas}
          className="circuit-canvas"
          tabIndex={0}
          onKeyDown={onKeyDown}
          onPointerMove={onMove}
          onPointerUp={onUp}
          onPointerCancel={() => setDrag(null)}
          onPointerDown={startCanvas}
          onContextMenu={(event) => event.preventDefault()}
          onDragOver={(event) => event.preventDefault()}
          onDrop={(event) => {
            event.preventDefault();
            const kind = event.dataTransfer.getData('text/plain') as EditorKind;
            if (PALETTE.includes(kind)) add(kind, toCanvas(event));
          }}
        >
          <defs>
            <pattern
              id="grid"
              width={GRID}
              height={GRID}
              patternUnits="userSpaceOnUse"
              patternTransform={`translate(${view.x} ${view.y}) scale(${view.zoom})`}
            >
              <circle cx="0" cy="0" r="0.8" className="grid-dot" />
            </pattern>
          </defs>
          <rect width="100%" height="100%" fill="url(#grid)" pointerEvents="none" />

          <g transform={`translate(${view.x} ${view.y}) scale(${view.zoom})`}>
            {elements.map((element) => (
              <Part
                key={element.id}
                element={element}
                selected={selection.includes(element.id)}
                onPointerDown={(event) => startMove(event, element)}
                onTerminal={(event, point) => startDraw(event, point)}
                onEnd={(event, end) => startEnd(event, element, end)}
              />
            ))}

            {drag?.type === 'marquee' && (
              <rect
                className="marquee"
                x={Math.min(drag.from.x, drag.to.x)}
                y={Math.min(drag.from.y, drag.to.y)}
                width={Math.abs(drag.to.x - drag.from.x)}
                height={Math.abs(drag.to.y - drag.from.y)}
                pointerEvents="none"
              />
            )}

            {drag?.type === 'draw' && (
              <line className="wire-ghost" x1={drag.from.x} y1={drag.from.y} x2={drag.to.x} y2={drag.to.y} />
            )}
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
            {elements.map((element) => (
              <PartLabel key={element.id} element={element} result={results.get(element.id)} shown={shown} />
            ))}
            {[...nodes.pointOf].map(([id, point]) => {
              const voltage = voltages.get(id);
              if (!voltage) return null;
              return (
                <div key={`node-${id}`} className="node-label" style={{ left: point.x + 6, top: point.y - 4 }}>
                  <Value label="φ" value={voltage} />
                </div>
              );
            })}
          </div>
        </div>

        {(failure || (elements.length > 0 && !nodes.grounded && nodes.byPoint.size > 0)) && (
          <div className={failure ? 'circuit-note error' : 'circuit-note'}>
            {failure ??
              t('circuit.noGround')}
          </div>
        )}
      </div>

      <aside className="circuit-inspector">
        {chosen ? (
          <Inspector
            element={chosen}
            result={results.get(chosen.id)}
            onChange={(change) => update(chosen.id, change)}
            onRemove={() => remove([chosen.id])}
          />
        ) : (
          <p className="panel-empty">
            {elements.length === 0
              ? t('circuit.empty')
              : t('circuit.choose')}
          </p>
        )}
      </aside>
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
    case 'wire':
      return <path d={`M${-HALF} 0H${HALF}`} />;
  }
}

interface PartProps {
  element: EditorElement;
  selected: boolean;
  onPointerDown(event: ReactPointerEvent): void;
  onTerminal(event: ReactPointerEvent, point: Point): void;
  onEnd(event: ReactPointerEvent, end: 'a' | 'b'): void;
}

function Part({ element, selected, onPointerDown, onTerminal, onEnd }: PartProps) {
  const places = terminals(element);
  const className = selected ? 'part selected' : 'part';

  if (element.kind === 'wire') {
    const { a, b } = places as Record<'a' | 'b', Point>;
    return (
      <g className={className}>
        <line className="hit" x1={a.x} y1={a.y} x2={b.x} y2={b.y} onPointerDown={onPointerDown} />
        <line className="wire" x1={a.x} y1={a.y} x2={b.x} y2={b.y} pointerEvents="none" />
        <circle className="end" cx={a.x} cy={a.y} r="5" onPointerDown={(event) => onEnd(event, 'a')} />
        <circle className="end" cx={b.x} cy={b.y} r="5" onPointerDown={(event) => onEnd(event, 'b')} />
      </g>
    );
  }

  return (
    <g className={className}>
      <g transform={`translate(${element.x} ${element.y}) rotate(${element.rotation * 90})`}>
        <g className="symbol">
          <Symbol kind={element.kind} />
        </g>
        <rect className="hit" x={-HALF + 8} y="-16" width={2 * HALF - 16} height="32" onPointerDown={onPointerDown} />
      </g>
      {(Object.entries(places) as [Terminal, Point][]).map(([name, point]) => (
        <circle
          key={name}
          className="terminal"
          cx={point.x}
          cy={point.y}
          r="5"
          onPointerDown={(event) => (element.kind === 'ground' ? onPointerDown(event) : onTerminal(event, point))}
        />
      ))}
    </g>
  );
}

type Side = 'below' | 'above' | 'right' | 'left';

/** Where a label sits against its point, so the block grows away from the part. */
const SIDES: Record<Side, string> = {
  below: 'translate(-50%, 0)',
  above: 'translate(-50%, -100%)',
  right: 'translate(0, -50%)',
  left: 'translate(-100%, -50%)',
};

/**
 * The ID and the voltage and current of a part, beside it: under a horizontal part, right of a
 * vertical one, on the other side if a potentiometer's wiper is there. An ideal wire has no drop,
 * so it only gets a label once it has a material or a resistance; the inspector shows the rest.
 */
function PartLabel({
  element,
  result,
  shown,
}: {
  element: EditorElement;
  result: CircuitResponse['elements'][number] | undefined;
  shown: Shown;
}) {
  if (element.kind === 'ground') return null;
  let at: Point;
  let side: Side;
  if (element.kind === 'wire') {
    if (!element.material && !element.rOverride?.trim()) return null;
    const { a, b } = terminals(element) as Record<'a' | 'b', Point>;
    at = { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
    side = Math.abs(b.x - a.x) >= Math.abs(b.y - a.y) ? 'above' : 'right';
    at = side === 'above' ? { x: at.x, y: at.y - 8 } : { x: at.x + 10, y: at.y };
  } else {
    const vertical = element.rotation % 2 === 1;
    const wiper = terminals(element).wiper;
    side = vertical ? 'right' : 'below';
    if (wiper && (vertical ? wiper.x > element.x : wiper.y > element.y)) side = vertical ? 'left' : 'above';
    const gap = 18;
    at = {
      x: element.x + (side === 'right' ? gap : side === 'left' ? -gap : 0),
      y: element.y + (side === 'below' ? gap : side === 'above' ? -gap : 0),
    };
  }
  return (
    <div className="part-label" style={{ left: at.x, top: at.y, transform: SIDES[side] }}>
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
  result,
  onChange,
  onRemove,
}: {
  element: EditorElement;
  result: CircuitResponse['elements'][number] | undefined;
  onChange(change: Partial<EditorElement>): void;
  onRemove(): void;
}) {
  const { t, language } = useT();
  const parameters = element as unknown as Record<string, string | undefined>;
  return (
    <div className="inspector">
      <div className="inspector-head">
        <span className="inspector-id">{element.id}</span>
        <span className="inspector-kind">{t(KIND_LABELS[element.kind])}</span>
        <button className="inspector-delete" title={`${t('circuit.delete')} (${t('key.delete')})`} onClick={onRemove}>
          ✕
        </button>
      </div>

      {element.kind === 'wire' && (
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

      {FIELDS[element.kind].map((field) => (
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
