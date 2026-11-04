import { useCallback, useEffect, useRef, useState } from 'react';
import type { CSSProperties } from 'react';
import { api, type GeometryKind, type GeometryObject, type GeometryViewResponse, type ViewPoint } from '../api';
import { useT } from '../i18n';

interface Props {
  /** The scene to draw; the engine builds and projects it. */
  objects: GeometryObject[];
}

/** The school view of `quantus_core::geometry::view::Camera::default`: 25° round, 20° up. */
export const DEFAULT_YAW = (25 * Math.PI) / 180;
export const DEFAULT_PITCH = (20 * Math.PI) / 180;
/** The share of the plot's half size that the unit circle of the view takes. */
const FILL = 0.92;
/** How far a label sits from its anchor, in pixels. */
const LABEL_OFFSET = 6;
/** Radians of turn per pixel dragged. */
const TURN = 0.01;
const ZOOM_STEP = 1.15;
/** The zoom on top of the fit stays in this range. */
const MIN_ZOOM = 0.2;
const MAX_ZOOM = 50;
const HALF_PI = Math.PI / 2;
/** The number of object colours, `--geometry-0` … in the stylesheet; they repeat after that. */
export const COLOURS = 8;

/** The colour of the object at `index` of the view, as a style that sets `--object`. */
export function colourStyle(index: number): CSSProperties {
  return { '--object': `var(--geometry-${index % COLOURS})` } as CSSProperties;
}

/** A name as the engine keys it, `P_1` or `\alpha`, as SVG text: the index lowered, a vector with its arrow. */
function Name({ name, kind }: { name: string; kind: GeometryKind | undefined }) {
  const [base, index] = name.replace(/\\/g, '').split('_', 2);
  return (
    <>
      {/* U+20D7, the combining arrow above, in the same run as the letter it sits on. */}
      <tspan>{kind === 'vector' ? `${base}\u20D7` : base}</tspan>
      {index && (
        <tspan className="geometry-index" dy={4}>
          {index}
        </tspan>
      )}
    </>
  );
}

/** How the scene is looked at; `zoom` is on top of the fit, so 1 shows the whole box. */
interface Look {
  yaw: number;
  pitch: number;
  zoom: number;
}

const START: Look = { yaw: DEFAULT_YAW, pitch: DEFAULT_PITCH, zoom: 1 };
/** From above: x to the right, y up, as `Camera::from_above`. */
const ABOVE: Omit<Look, 'zoom'> = { yaw: -HALF_PI, pitch: HALF_PI };

/**
 * The 3D view of the scene as a hand-drawn SVG, like `FunctionPlot`: the engine projects the
 * scene into flat primitives, far to near, and this only draws them in that order. Dragging
 * turns the scene, the wheel zooms.
 */
export function GeometryPlot({ objects }: Props) {
  const { t } = useT();
  const host = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ width: 600, height: 460 });
  const [view, setView] = useState<GeometryViewResponse | null>(null);
  const [look, setLook] = useState<Look>(START);
  const drag = useRef<{ x: number; y: number; look: Look } | null>(null);

  useEffect(() => {
    const element = host.current;
    if (!element) return;
    const observer = new ResizeObserver(([entry]) => {
      const { width, height } = entry.contentRect;
      if (width > 0 && height > 0) setSize({ width, height });
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, []);

  // One request at a time: while the engine projects, moves only update `wanted`, and the
  // answer asks again for the latest of them. A drag thus never queues up stale pictures.
  const wanted = useRef({ objects, look });
  wanted.current = { objects, look };
  const busy = useRef(false);
  const stale = useRef(false);
  const alive = useRef(true);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);
  const project = useCallback(() => {
    if (busy.current) {
      stale.current = true;
      return;
    }
    busy.current = true;
    stale.current = false;
    const { objects: scene, look: at } = wanted.current;
    api
      .geometryView({ scene: { objects: scene }, ...at })
      .then(
        (next) => {
          if (alive.current) setView(next);
        },
        // The last picture stays; the next change asks again.
        () => {},
      )
      .finally(() => {
        busy.current = false;
        if (stale.current && alive.current) project();
      });
  }, []);
  useEffect(() => project(), [objects, look, project]);

  // Zoom with the wheel. A native listener, since React's wheel listeners are passive and could
  // not keep the page from scrolling along.
  useEffect(() => {
    const element = host.current;
    if (!element) return;
    const onWheel = (event: WheelEvent) => {
      event.preventDefault();
      const factor = event.deltaY > 0 ? 1 / ZOOM_STEP : ZOOM_STEP;
      setLook((l) => ({ ...l, zoom: Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, l.zoom * factor)) }));
    };
    element.addEventListener('wheel', onWheel, { passive: false });
    return () => element.removeEventListener('wheel', onWheel);
  }, []);

  const onPointerDown = (event: React.PointerEvent) => {
    drag.current = { x: event.clientX, y: event.clientY, look };
    (event.currentTarget as Element).setPointerCapture(event.pointerId);
  };
  // Dragging right turns the scene to the right, dragging down tips it towards the viewer.
  const onPointerMove = (event: React.PointerEvent) => {
    const start = drag.current;
    if (!start) return;
    const yaw = start.look.yaw - (event.clientX - start.x) * TURN;
    const pitch = Math.min(HALF_PI, Math.max(-HALF_PI, start.look.pitch + (event.clientY - start.y) * TURN));
    setLook({ ...start.look, yaw, pitch });
  };
  const onPointerUp = () => {
    drag.current = null;
  };

  const { width, height } = size;
  const scale = (Math.min(width, height) / 2) * FILL;
  const px = ([x, y]: ViewPoint) => [width / 2 + x * scale, height / 2 - y * scale] as const;
  const points = (list: ViewPoint[]) =>
    list
      .map((p) => px(p))
      .map(([x, y]) => `${x.toFixed(1)},${y.toFixed(1)}`)
      .join(' ');

  return (
    <div className="plot geometry-plot">
      <div
        className="plot-canvas"
        ref={host}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerLeave={onPointerUp}
      >
        <svg width={width} height={height} viewBox={`0 0 ${width} ${height}`}>
          {view?.primitives.map((primitive, i) => {
            switch (primitive.kind) {
              case 'segment': {
                const [x1, y1] = px(primitive.from);
                const [x2, y2] = px(primitive.to);
                const style = primitive.object === null ? undefined : colourStyle(primitive.object);
                return <line key={i} className={`geometry-${primitive.stroke}`} style={style} x1={x1} y1={y1} x2={x2} y2={y2} />;
              }
              case 'polygon':
                return <polygon key={i} className="geometry-face" style={colourStyle(primitive.object)} points={points(primitive.points)} />;
              case 'marker': {
                const [cx, cy] = px(primitive.at);
                return <circle key={i} className="geometry-marker" style={colourStyle(primitive.object)} cx={cx} cy={cy} r={4} />;
              }
              case 'label': {
                const [x, y] = px(primitive.at);
                if (primitive.object === null) {
                  return (
                    <text key={i} className="geometry-tick-label" x={x + LABEL_OFFSET} y={y - LABEL_OFFSET}>
                      {primitive.text}
                    </text>
                  );
                }
                return (
                  <text key={i} className="geometry-label" style={colourStyle(primitive.object)} x={x + LABEL_OFFSET} y={y - LABEL_OFFSET}>
                    <Name name={primitive.text} kind={view.objects[primitive.object]?.kind} />
                  </text>
                );
              }
            }
          })}
        </svg>
      </div>
      <div className="plot-footer">
        <span className="plot-hint">{t('geometry.plot.hint')}</span>
        <span className="geometry-plot-buttons">
          <button className="copy-btn" onClick={() => setLook((l) => ({ ...l, ...ABOVE }))}>
            {t('geometry.plot.above')}
          </button>
          <button className="copy-btn" onClick={() => setLook(START)}>
            {t('analysis.reset')}
          </button>
        </span>
      </div>
    </div>
  );
}
