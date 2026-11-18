import {
  useCallback,
  useEffect,
  useId,
  useState,
  type CSSProperties,
  type ReactNode,
  type PointerEvent as ReactPointerEvent,
  type RefObject,
} from 'react';
import { createPortal } from 'react-dom';
import { useT } from '../i18n';
import type { Placement } from '../labelLayout';
import {
  corner,
  GRID,
  inRectangle,
  inside,
  movedBy,
  pointKey,
  rotatedQuarter,
  snap,
  type Placeable,
  type Point,
  type Rect,
} from '../schematic';

/**
 * The parts of a schematic editor that are not about what is drawn: the canvas that pans and
 * zooms, the grid, selecting, moving, turning and deleting parts, drawing wires from pins, the
 * labels measured in the browser. The electronics mode's `CircuitEditor` and the Logic mode's
 * `LogicEditor` are built on them.
 */

/** The drawing is shown scaled by `zoom` and then moved by (x, y) on the canvas. */
export interface View {
  x: number;
  y: number;
  zoom: number;
}

export const HOME: View = { x: 0, y: 0, zoom: 1 };
const MIN_ZOOM = 0.25;
const MAX_ZOOM = 4;

/** The SVG transform of the drawing in `view`. */
export const viewTransform = (view: View) => `translate(${view.x} ${view.y}) scale(${view.zoom})`;

/** `v` scaled by `factor` (within the zoom's limits), the canvas point `at` staying where it is. */
function zoomedAbout(v: View, factor: number, at: Point): View {
  const zoom = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, v.zoom * factor));
  return { zoom, x: at.x - ((at.x - v.x) * zoom) / v.zoom, y: at.y - ((at.y - v.y) * zoom) / v.zoom };
}

/**
 * How the drawing lies on the canvas. The wheel zooms about the pointer (with Ctrl too, as a
 * touchpad pinch arrives); with Shift, or a sideways touchpad swipe, it moves the drawing. React's
 * wheel handler is passive and could not keep the webview from zooming or scrolling the whole page,
 * so this one is native.
 */
export function useCanvasView(canvas: RefObject<SVGSVGElement | null>) {
  const [view, setView] = useState<View>(HOME);

  useEffect(() => {
    const svg = canvas.current;
    if (!svg) return;
    const onWheel = (event: WheelEvent) => {
      event.preventDefault();
      const unit = event.deltaMode === WheelEvent.DOM_DELTA_LINE ? 16 : 1;
      if (event.shiftKey || (event.deltaX !== 0 && !event.ctrlKey)) {
        const sideways = event.shiftKey && event.deltaX === 0;
        const dx = (sideways ? event.deltaY : event.deltaX) * unit;
        const dy = (event.shiftKey ? 0 : event.deltaY) * unit;
        setView((v) => ({ ...v, x: v.x - dx, y: v.y - dy }));
        return;
      }
      const box = svg.getBoundingClientRect();
      const at = { x: event.clientX - box.left, y: event.clientY - box.top };
      setView((v) => zoomedAbout(v, Math.exp(-event.deltaY * unit * 0.002), at));
    };
    svg.addEventListener('wheel', onWheel, { passive: false });
    return () => svg.removeEventListener('wheel', onWheel);
  }, [canvas]);

  /** Zooms by `factor` about the middle of the canvas. */
  const zoomBy = (factor: number) => {
    const svg = canvas.current;
    if (!svg) return;
    const { width, height } = svg.getBoundingClientRect();
    setView((v) => zoomedAbout(v, factor, { x: width / 2, y: height / 2 }));
  };

  /** The point under the pointer in drawing coordinates, unsnapped. */
  const toDrawing = (event: { clientX: number; clientY: number }): Point => {
    const box = canvas.current!.getBoundingClientRect();
    return { x: (event.clientX - box.left - view.x) / view.zoom, y: (event.clientY - box.top - view.y) / view.zoom };
  };

  /** The grid point under the pointer. */
  const toGrid = (event: { clientX: number; clientY: number }): Point => {
    const at = toDrawing(event);
    return { x: snap(at.x), y: snap(at.y) };
  };

  /** Shows `box` of the drawing whole and centred, at most a little enlarged. */
  const fit = (box: Rect, margin = 30) => {
    const svg = canvas.current;
    if (!svg || !Number.isFinite(box.w) || !Number.isFinite(box.h)) return;
    const { width, height } = svg.getBoundingClientRect();
    if (!width || !height) return;
    const zoom = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, Math.min((width - 2 * margin) / Math.max(box.w, 1), (height - 2 * margin) / Math.max(box.h, 1), 1.5)));
    setView({ zoom, x: width / 2 - (box.x + box.w / 2) * zoom, y: height / 2 - (box.y + box.h / 2) * zoom });
  };

  return { view, setView, toDrawing, toGrid, fit, zoomBy };
}

/**
 * The labels are laid out from their real sizes, which only the browser knows once KaTeX has set
 * them. `measure` is the ref of a label element with `data-label` set to its id; offset sizes
 * ignore the zoom transform, so they are already in drawing units.
 */
export function useLabelSizes() {
  const [sizes, setSizes] = useState<Map<string, { w: number; h: number }>>(() => new Map());
  const [observer] = useState(
    () =>
      new ResizeObserver((entries) =>
        setSizes((previous) => {
          let next = previous;
          for (const entry of entries) {
            const target = entry.target as HTMLElement;
            const id = target.dataset.label!;
            const size = { w: target.offsetWidth, h: target.offsetHeight };
            const old = previous.get(id);
            if (old && old.w === size.w && old.h === size.h) continue;
            if (next === previous) next = new Map(previous);
            next.set(id, size);
          }
          return next;
        }),
      ),
  );
  const measure = useCallback(
    (label: HTMLDivElement | null) => {
      if (!label) return;
      observer.observe(label);
      return () => observer.unobserve(label);
    },
    [observer],
  );
  return { sizes, measure };
}

/** Where a label is drawn: at its placement once measured, until then hidden at its anchor. */
export function labelStyle(placement: Placement | undefined, anchor: Point): CSSProperties {
  return placement ? { left: placement.x, top: placement.y } : { left: anchor.x, top: anchor.y, visibility: 'hidden' };
}

/** How much one press of a zoom button or key zooms. */
export const ZOOM_STEP = 1.25;

/**
 * The zoom keys of a focused canvas: + (or =) and − zoom, 0 shows the whole drawing. Ctrl+0 and
 * Ctrl+Plus stay the app's own. Returns whether the key was one of them.
 */
export function zoomKey(event: { key: string; ctrlKey: boolean; metaKey: boolean; altKey: boolean }, view: ReturnType<typeof useCanvasView>, fit: () => void): boolean {
  if (event.ctrlKey || event.metaKey || event.altKey) return false;
  if (event.key === '+' || event.key === '=') view.zoomBy(ZOOM_STEP);
  else if (event.key === '-') view.zoomBy(1 / ZOOM_STEP);
  else if (event.key === '0') fit();
  else return false;
  return true;
}

/**
 * An editor filling the window: `portal` puts it into the page's body while `full`, so that nothing
 * of the app is drawn over it. Esc leaves, unless the editor used it (to clear a selection).
 */
export function useFullscreen() {
  const [full, setFull] = useState(false);
  useEffect(() => {
    if (!full) return;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape' && !event.defaultPrevented) setFull(false);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [full]);
  const portal = (editor: ReactNode) => (full ? createPortal(editor, document.body) : editor);
  return { full, setFull, portal };
}

export type Fullscreen = ReturnType<typeof useFullscreen>;

/**
 * −, the zoom in percent (a click goes back to 100 %), + and Fit, in the canvas's corner, and the
 * full-screen switch when the editor has one.
 */
export function ZoomControls({ view, fit, fullscreen }: { view: ReturnType<typeof useCanvasView>; fit(): void; fullscreen?: Fullscreen }) {
  const { t } = useT();
  return (
    <div className="zoom-controls" onPointerDown={(event) => event.stopPropagation()}>
      <button className="zoom-button" title={t('schematic.zoomOut')} aria-label={t('schematic.zoomOut')} onClick={() => view.zoomBy(1 / ZOOM_STEP)}>
        −
      </button>
      <button className="zoom-button zoom-level" title={t('schematic.zoomReset')} onClick={() => view.zoomBy(1 / view.view.zoom)}>
        {Math.round(view.view.zoom * 100)} %
      </button>
      <button className="zoom-button" title={t('schematic.zoomIn')} aria-label={t('schematic.zoomIn')} onClick={() => view.zoomBy(ZOOM_STEP)}>
        +
      </button>
      <button className="zoom-button" title={t('schematic.fit')} onClick={fit}>
        {t('schematic.fit.short')}
      </button>
      {fullscreen && (
        <button
          className={fullscreen.full ? 'zoom-button active' : 'zoom-button'}
          title={t(fullscreen.full ? 'schematic.fullscreen.exit' : 'schematic.fullscreen')}
          aria-label={t(fullscreen.full ? 'schematic.fullscreen.exit' : 'schematic.fullscreen')}
          aria-pressed={fullscreen.full}
          onClick={() => fullscreen.setFull(!fullscreen.full)}
        >
          {fullscreen.full ? '⤡' : '⤢'}
        </button>
      )}
    </div>
  );
}

/** The dotted grid behind the drawing, moving and scaling with it. */
export function GridBackground({ view }: { view: View }) {
  // React's ids hold characters that `url(#…)` may not take.
  const id = `grid-${useId().replace(/[^A-Za-z0-9_-]/g, '')}`;
  return (
    <>
      <defs>
        <pattern id={id} width={GRID} height={GRID} patternUnits="userSpaceOnUse" patternTransform={viewTransform(view)}>
          <circle cx="0" cy="0" r="0.8" className="grid-dot" />
        </pattern>
      </defs>
      <rect width="100%" height="100%" fill={`url(#${id})`} pointerEvents="none" />
    </>
  );
}

export type Drag<E> =
  /** `moved`: whether the pointer has left the grid point it started on; if not, it was a tap. */
  | { type: 'move'; from: Point; origins: Map<string, E>; moved: boolean }
  | { type: 'end'; id: string; end: 'a' | 'b' }
  /** `across`: whether the wire leaves sideways first, decided by the first step of the pointer. */
  | { type: 'draw'; from: Point; to: Point; across: boolean | null }
  | { type: 'marquee'; from: Point; to: Point }
  | { type: 'pan'; from: Point; origin: View };

interface Editing<E extends Placeable> {
  elements: E[];
  onChange(next: E[]): void;
  canvas: RefObject<SVGSVGElement | null>;
  view: ReturnType<typeof useCanvasView>;
  /** The pins of a part (both ends of a wire) at their places on the canvas. */
  pins(element: E): Point[];
  /** A new wire from `from` to `to` among `elements`. */
  newWire(elements: E[], from: Point, to: Point): E;
  /** A part's rotation after a quarter turn. */
  turned?(rotation: number): number;
  /** A part pressed and let go without moving it, such as a switch flipped. */
  onTap?(element: E): void;
}

/**
 * Selecting, moving, turning and deleting parts, pulling a wire's end, drawing a wire from a pin
 * (an L of up to two wires), the selection rectangle and moving the canvas.
 */
export function useSchematicEditing<E extends Placeable>({ elements, onChange, canvas, view: canvasView, pins, newWire, turned, onTap }: Editing<E>) {
  const [selection, setSelection] = useState<string[]>([]);
  const [drag, setDrag] = useState<Drag<E> | null>(null);
  const { view, setView, toDrawing, toGrid } = canvasView;

  const update = (id: string, change: Partial<E>) => onChange(elements.map((e) => (e.id === id ? { ...e, ...change } : e)));

  const remove = (ids: string[]) => {
    onChange(elements.filter((e) => !ids.includes(e.id)));
    setSelection([]);
  };

  const rotateSelected = () => {
    if (!elements.some((e) => selection.includes(e.id))) return;
    onChange(rotatedQuarter(elements, selection, pins, turned));
  };

  const capture = (event: ReactPointerEvent) => canvas.current?.setPointerCapture(event.pointerId);

  // Only the left button works on parts; the others fall through to the canvas and move it.
  const startMove = (event: ReactPointerEvent, element: E) => {
    if (event.button !== 0) return;
    event.stopPropagation();
    capture(event);
    // Grabbing a part of the selection moves all of it; any other part becomes the selection.
    const group = selection.includes(element.id) ? selection : [element.id];
    setSelection(group);
    setDrag({ type: 'move', from: toGrid(event), origins: new Map(elements.filter((e) => group.includes(e.id)).map((e) => [e.id, e])), moved: false });
  };

  const startEnd = (event: ReactPointerEvent, element: E, end: 'a' | 'b') => {
    if (event.button !== 0) return;
    event.stopPropagation();
    capture(event);
    setSelection([element.id]);
    setDrag({ type: 'end', id: element.id, end });
  };

  const startDraw = (event: ReactPointerEvent, from: Point) => {
    if (event.button !== 0) return;
    event.stopPropagation();
    capture(event);
    setDrag({ type: 'draw', from, to: from, across: null });
  };

  /** Left on empty canvas draws a selection rectangle; the middle or right button moves the canvas. */
  const startCanvas = (event: ReactPointerEvent) => {
    capture(event);
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
    const at = toGrid(event);
    if (drag.type === 'draw') {
      if (at.x === drag.to.x && at.y === drag.to.y) return;
      const dx = Math.abs(at.x - drag.from.x);
      const dy = Math.abs(at.y - drag.from.y);
      const across = dx === 0 && dy === 0 ? null : (drag.across ?? dx >= dy);
      setDrag({ ...drag, to: at, across });
    } else if (drag.type === 'end') {
      // The end moves along the axis it is pulled most, so the wire stays across or down.
      const wire = elements.find((e) => e.id === drag.id);
      if (!wire) return;
      const other = drag.end === 'a' ? { x: wire.x2 ?? wire.x, y: wire.y2 ?? wire.y } : { x: wire.x, y: wire.y };
      const end = Math.abs(at.x - other.x) >= Math.abs(at.y - other.y) ? { x: at.x, y: other.y } : { x: other.x, y: at.y };
      update(drag.id, (drag.end === 'a' ? end : { x2: end.x, y2: end.y }) as Partial<E>);
    } else {
      const [dx, dy] = [at.x - drag.from.x, at.y - drag.from.y];
      if (!drag.moved && dx === 0 && dy === 0) return;
      if (!drag.moved) setDrag({ ...drag, moved: true });
      onChange(movedBy(elements, drag.origins, dx, dy));
    }
  };

  const onUp = () => {
    if (drag?.type === 'draw' && pointKey(drag.from) !== pointKey(drag.to)) {
      const bend = corner(drag.from, drag.to, drag.across);
      const next = [...elements];
      for (const [from, to] of [
        [drag.from, bend],
        [bend, drag.to],
      ]) {
        if (pointKey(from) === pointKey(to)) continue;
        next.push(newWire(next, from, to));
      }
      onChange(next);
      setSelection(next.slice(elements.length).map((e) => e.id));
    } else if (drag?.type === 'marquee') {
      setSelection(inRectangle(elements, drag.from, drag.to, pins));
    } else if (drag?.type === 'move' && !drag.moved && drag.origins.size === 1 && onTap) {
      onTap([...drag.origins.values()][0]);
    }
    setDrag(null);
  };

  return { selection, setSelection, drag, setDrag, update, remove, rotateSelected, startMove, startEnd, startDraw, startCanvas, onMove, onUp };
}

/** What a drag draws over the drawing: the selection rectangle, or the wire about to be drawn. */
export function DragGhost<E>({ drag }: { drag: Drag<E> | null }) {
  if (drag?.type === 'marquee') {
    return (
      <rect
        className="marquee"
        x={Math.min(drag.from.x, drag.to.x)}
        y={Math.min(drag.from.y, drag.to.y)}
        width={Math.abs(drag.to.x - drag.from.x)}
        height={Math.abs(drag.to.y - drag.from.y)}
        pointerEvents="none"
      />
    );
  }
  if (drag?.type === 'draw') {
    return <polyline className="wire-ghost" points={[drag.from, corner(drag.from, drag.to, drag.across), drag.to].map((p) => `${p.x},${p.y}`).join(' ')} />;
  }
  return null;
}

interface WireProps {
  a: Point;
  b: Point;
  className: string;
  /** Extra classes of the drawn line, such as the value it carries. */
  lineClassName?: string;
  /** Whether a wire may start in its middle (an ideal wire), making a T-junction. */
  joins: boolean;
  /** The points where pins meet; their dots are hidden until hovered. */
  connected: Set<string>;
  onPointerDown(event: ReactPointerEvent): void;
  onEnd(event: ReactPointerEvent, end: 'a' | 'b'): void;
  onTerminal(event: ReactPointerEvent, point: Point): void;
}

/**
 * A wire: grabbed to move it, its ends pulled. Near a grid point in its middle a joining wire
 * offers a handle, as a pin does: a wire drawn from it starts a T-junction.
 */
export function WirePart({ a, b, className, lineClassName, joins, connected, onPointerDown, onEnd, onTerminal }: WireProps) {
  /** The grid point on the wire under the pointer, where a new wire can start. */
  const [junction, setJunction] = useState<Point | null>(null);
  const dot = (point: Point) => (connected.has(pointKey(point)) ? 'end connected' : 'end');
  const findJunction = (event: ReactPointerEvent<SVGElement>) => {
    const matrix = (event.currentTarget as SVGGraphicsElement).getScreenCTM()?.inverse();
    if (!matrix || !joins) return setJunction(null);
    const at = new DOMPoint(event.clientX, event.clientY).matrixTransform(matrix);
    const point = { x: snap(at.x), y: snap(at.y) };
    const near = Math.hypot(at.x - point.x, at.y - point.y) <= 6 && inside(a, b, point);
    setJunction((old) => (near ? (old && pointKey(old) === pointKey(point) ? old : point) : null));
  };
  return (
    <g className={className} onPointerMove={findJunction} onPointerLeave={() => setJunction(null)}>
      <line className="hit" x1={a.x} y1={a.y} x2={b.x} y2={b.y} onPointerDown={onPointerDown} />
      <line className={lineClassName ? `wire ${lineClassName}` : 'wire'} x1={a.x} y1={a.y} x2={b.x} y2={b.y} pointerEvents="none" />
      <circle className={dot(a)} cx={a.x} cy={a.y} r="5" onPointerDown={(event) => onEnd(event, 'a')} />
      <circle className={dot(b)} cx={b.x} cy={b.y} r="5" onPointerDown={(event) => onEnd(event, 'b')} />
      {junction && (
        <circle
          className="end"
          cx={junction.x}
          cy={junction.y}
          r="5"
          onPointerDown={(event) => {
            setJunction(null);
            onTerminal(event, junction);
          }}
        />
      )}
    </g>
  );
}
