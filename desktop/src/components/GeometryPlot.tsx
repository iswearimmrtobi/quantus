import { useEffect, useRef, useState } from 'react';
import { api, type GeometryObject, type GeometryViewResponse, type ViewPoint } from '../api';

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

/**
 * The 3D view of the scene as a hand-drawn SVG, like `FunctionPlot`: the engine projects the
 * scene into flat primitives, far to near, and this only draws them in that order.
 */
export function GeometryPlot({ objects }: Props) {
  const host = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ width: 600, height: 460 });
  const [view, setView] = useState<GeometryViewResponse | null>(null);
  const request = useRef(0);

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

  useEffect(() => {
    const id = ++request.current;
    api.geometryView({ scene: { objects }, yaw: DEFAULT_YAW, pitch: DEFAULT_PITCH, zoom: 1 }).then(
      (next) => {
        if (id === request.current) setView(next);
      },
      // The last picture stays; the next change asks again.
      () => {},
    );
  }, [objects]);

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
      <div className="plot-canvas" ref={host}>
        <svg width={width} height={height} viewBox={`0 0 ${width} ${height}`}>
          {view?.primitives.map((primitive, i) => {
            switch (primitive.kind) {
              case 'segment': {
                const [x1, y1] = px(primitive.from);
                const [x2, y2] = px(primitive.to);
                return <line key={i} className={`geometry-${primitive.stroke}`} x1={x1} y1={y1} x2={x2} y2={y2} />;
              }
              case 'polygon':
                return <polygon key={i} className="geometry-face" points={points(primitive.points)} />;
              case 'marker': {
                const [cx, cy] = px(primitive.at);
                return <circle key={i} className="geometry-marker" cx={cx} cy={cy} r={4} />;
              }
              case 'label': {
                const [x, y] = px(primitive.at);
                const axis = primitive.object === null;
                return (
                  <text key={i} className={axis ? 'geometry-tick-label' : 'geometry-label'} x={x + LABEL_OFFSET} y={y - LABEL_OFFSET}>
                    {primitive.text}
                  </text>
                );
              }
            }
          })}
        </svg>
      </div>
    </div>
  );
}
