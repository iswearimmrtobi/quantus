import { useEffect, useId, useRef, useState, type ReactNode, type RefObject } from 'react';
import { createPortal } from 'react-dom';
import { useT } from '../i18n';
import { GRID, snap, type Point, type Rect } from './geometry';

/**
 * The canvas of a schematic editor: the drawing panned and zoomed on it, the zoom controls, the
 * dotted grid, and the editor filling the window.
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

/** How much one press of a zoom button or key zooms. */
export const ZOOM_STEP = 1.25;

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
export function ZoomControls({
  view,
  fit,
  fullscreen,
  history,
}: {
  view: ReturnType<typeof useCanvasView>;
  fit(): void;
  fullscreen?: Fullscreen;
  /** Undo and redo, before the zoom. */
  history?: { undo(): void; redo(): void; canUndo: boolean; canRedo: boolean };
}) {
  const { t } = useT();
  return (
    <div className="zoom-controls" onPointerDown={(event) => event.stopPropagation()}>
      {history && (
        <>
          <button className="zoom-button" title={t('schematic.undo')} aria-label={t('schematic.undo')} disabled={!history.canUndo} onClick={() => history.undo()}>
            ↶
          </button>
          <button className="zoom-button zoom-gap" title={t('schematic.redo')} aria-label={t('schematic.redo')} disabled={!history.canRedo} onClick={() => history.redo()}>
            ↷
          </button>
        </>
      )}
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

/**
 * Shows the whole drawing on opening and whenever `fitKey` changes, as after a circuit was loaded,
 * drawn or arranged anew: once the canvas has its size and every label has been `measured`, so
 * that nothing is cut off at the edge. `layout` is what changes as the labels are laid out.
 */
export function useFitOnRequest(canvas: RefObject<SVGSVGElement | null>, fitKey: unknown, measured: boolean, fit: () => void, layout: unknown) {
  const pending = useRef(true);
  const [tries, setTries] = useState(0);
  useEffect(() => {
    pending.current = true;
  }, [fitKey]);
  useEffect(() => {
    if (!pending.current) return;
    const width = canvas.current?.getBoundingClientRect().width ?? 0;
    if ((!width || !measured) && tries < 30) {
      // Not laid out yet: look again in the next frame, a few times at most.
      const frame = requestAnimationFrame(() => setTries((n) => n + 1));
      return () => cancelAnimationFrame(frame);
    }
    pending.current = false;
    setTries(0);
    fit();
    // Only when asked, not on every change of the drawing.
  }, [fitKey, layout, tries]);
}
