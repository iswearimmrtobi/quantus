import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { api, type Asymptote, type Line, type MarkedPoint, type PlotData, type Segment } from '../api';
import { useT } from '../i18n';
import { Katex } from './Katex';

/** The visible part of the plane. */
interface View {
  xMin: number;
  xMax: number;
  yMin: number;
  yMax: number;
}

interface Props {
  /** The function as analysed; panning asks the engine for fresh segments of it. */
  latex: string;
  plot: PlotData;
  points: MarkedPoint[];
  asymptotes: Asymptote[];
  /** Tangent and normal, drawn in the accent colour. */
  lines?: Line[];
  /** Areas to shade down to the x-axis. */
  regions?: { segments: Segment[]; positive: boolean }[];
  /** Extra dots: the rows of a value table. */
  dots?: [number, number][];
  selected: number | null;
  onSelect(index: number | null): void;
}

const ZOOM_STEP = 1.15;
const RESAMPLE_DELAY = 120;

/** 1, 2 or 5 times a power of ten, so that about `count` ticks fit into `span`. */
function tickStep(span: number, count: number): number {
  const raw = span / count;
  const magnitude = 10 ** Math.floor(Math.log10(raw));
  const unit = raw / magnitude;
  return (unit < 1.5 ? 1 : unit < 3.5 ? 2 : unit < 7.5 ? 5 : 10) * magnitude;
}

function ticks(min: number, max: number, count: number): number[] {
  const step = tickStep(max - min, count);
  const out: number[] = [];
  for (let v = Math.ceil(min / step) * step; v <= max + step * 1e-9; v += step) out.push(Math.abs(v) < step * 1e-9 ? 0 : v);
  return out;
}

/**
 * The graph as a hand-drawn SVG (no chart library, like the circuit editor): grid, axes with
 * ticks, the curve, dashed asymptotes and the marked points. Wheel zooms around the cursor, drag
 * moves the view; each change asks the engine for segments that fit the new view.
 */
export function FunctionPlot({ latex, plot, points, asymptotes, lines = [], regions = [], dots = [], selected, onSelect }: Props) {
  const { t, language } = useT();
  const host = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ width: 600, height: 420 });
  const initial: View = useMemo(() => ({ xMin: plot.xMin, xMax: plot.xMax, yMin: plot.yMin, yMax: plot.yMax }), [plot]);
  const [view, setView] = useState<View>(initial);
  const [segments, setSegments] = useState<Segment[]>(plot.segments);
  const [hovered, setHovered] = useState<number | null>(null);
  const drag = useRef<{ x: number; y: number; view: View } | null>(null);
  const request = useRef(0);

  // A new analysis starts from its own window.
  useEffect(() => {
    setView(initial);
    setSegments(plot.segments);
  }, [initial, plot.segments]);

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

  // Fresh segments for a moved or zoomed view, once the movement pauses.
  useEffect(() => {
    if (view === initial) return;
    const id = ++request.current;
    const timer = window.setTimeout(() => {
      api.sampleFunction(latex, view, Math.round(size.width)).then(
        (sampled) => {
          if (id === request.current) setSegments(sampled.segments);
        },
        () => {},
      );
    }, RESAMPLE_DELAY);
    return () => window.clearTimeout(timer);
  }, [latex, view, initial, size.width]);

  const { width, height } = size;
  const sx = useCallback((x: number) => ((x - view.xMin) / (view.xMax - view.xMin)) * width, [view, width]);
  const sy = useCallback((y: number) => height - ((y - view.yMin) / (view.yMax - view.yMin)) * height, [view, height]);
  const path = (segment: Segment) => segment.map(([x, y]) => `${sx(x).toFixed(1)},${sy(y).toFixed(1)}`).join(' ');
  const label = (v: number) => {
    const text = String(Number(v.toPrecision(10)));
    return language === 'de' ? text.replace('.', ',') : text;
  };

  // Zoom around the cursor. A native listener, since React's wheel listeners are passive and
  // could not keep the page from scrolling along.
  useEffect(() => {
    const element = host.current;
    if (!element) return;
    const onWheel = (event: WheelEvent) => {
      event.preventDefault();
      const rect = element.getBoundingClientRect();
      const fx = (event.clientX - rect.left) / rect.width;
      const fy = 1 - (event.clientY - rect.top) / rect.height;
      const factor = event.deltaY > 0 ? ZOOM_STEP : 1 / ZOOM_STEP;
      setView((v) => {
        const cx = v.xMin + fx * (v.xMax - v.xMin);
        const cy = v.yMin + fy * (v.yMax - v.yMin);
        return {
          xMin: cx - (cx - v.xMin) * factor,
          xMax: cx + (v.xMax - cx) * factor,
          yMin: cy - (cy - v.yMin) * factor,
          yMax: cy + (v.yMax - cy) * factor,
        };
      });
    };
    element.addEventListener('wheel', onWheel, { passive: false });
    return () => element.removeEventListener('wheel', onWheel);
  }, []);

  const reset = () => {
    request.current++;
    setView(initial);
    setSegments(plot.segments);
  };

  const onPointerDown = (event: React.PointerEvent) => {
    if ((event.target as Element).closest('.plot-point')) return;
    drag.current = { x: event.clientX, y: event.clientY, view };
    (event.currentTarget as Element).setPointerCapture(event.pointerId);
  };
  const onPointerMove = (event: React.PointerEvent) => {
    const start = drag.current;
    if (!start) return;
    const dx = ((event.clientX - start.x) / width) * (start.view.xMax - start.view.xMin);
    const dy = ((event.clientY - start.y) / height) * (start.view.yMax - start.view.yMin);
    setView({ xMin: start.view.xMin - dx, xMax: start.view.xMax - dx, yMin: start.view.yMin + dy, yMax: start.view.yMax + dy });
  };
  const onPointerUp = () => {
    drag.current = null;
  };

  /** A line y = mx + b across the view, or x = c. */
  const straight = (line: { slope: number | null; intercept: number | null; x: number | null }) =>
    line.x !== null
      ? { x1: sx(line.x), y1: 0, x2: sx(line.x), y2: height }
      : { x1: 0, y1: sy((line.slope ?? 0) * view.xMin + (line.intercept ?? 0)), x2: width, y2: sy((line.slope ?? 0) * view.xMax + (line.intercept ?? 0)) };

  const xTicks = ticks(view.xMin, view.xMax, Math.max(4, Math.round(width / 80)));
  const yTicks = ticks(view.yMin, view.yMax, Math.max(4, Math.round(height / 60)));
  const shown = hovered ?? selected;

  return (
    <div className="plot">
      <div
        className="plot-canvas"
        ref={host}
        onPointerDown={onPointerDown}
        onPointerMove={onPointerMove}
        onPointerUp={onPointerUp}
        onPointerLeave={onPointerUp}
      >
        <svg width={width} height={height} viewBox={`0 0 ${width} ${height}`}>
          <defs>
            <clipPath id="plot-area">
              <rect x={0} y={0} width={width} height={height} />
            </clipPath>
          </defs>
          {xTicks.map((v) => (
            <line key={`gx${v}`} className="plot-grid" x1={sx(v)} y1={0} x2={sx(v)} y2={height} />
          ))}
          {yTicks.map((v) => (
            <line key={`gy${v}`} className="plot-grid" x1={0} y1={sy(v)} x2={width} y2={sy(v)} />
          ))}
          <g clipPath="url(#plot-area)">
            {regions.map((region, i) =>
              region.segments.map((segment, j) => (
                <polygon
                  key={`r${i}-${j}`}
                  className={region.positive ? 'plot-region' : 'plot-region negative'}
                  points={`${sx(segment[0][0])},${sy(0)} ${path(segment)} ${sx(segment[segment.length - 1][0])},${sy(0)}`}
                />
              )),
            )}
            {view.yMin < 0 && view.yMax > 0 && <line className="plot-axis" x1={0} y1={sy(0)} x2={width} y2={sy(0)} />}
            {view.xMin < 0 && view.xMax > 0 && <line className="plot-axis" x1={sx(0)} y1={0} x2={sx(0)} y2={height} />}
            {asymptotes.map((a, i) => (
              <line key={`a${i}`} className="plot-asymptote" {...straight(a)} />
            ))}
            {lines.map((line, i) => (
              <line key={`l${i}`} className="plot-line" {...straight(line)} />
            ))}
            {segments.map((segment, i) => (
              <polyline key={`s${i}`} className="plot-curve" points={path(segment)} />
            ))}
            {dots.map(([x, y], i) => (
              <circle key={`d${i}`} className="plot-dot" cx={sx(x)} cy={sy(y)} r={2.5} />
            ))}
          </g>
          {xTicks.map((v) =>
            v === 0 ? null : (
              <text key={`tx${v}`} className="plot-tick" x={sx(v)} y={Math.min(Math.max(sy(0) + 14, 14), height - 4)} textAnchor="middle">
                {label(v)}
              </text>
            ),
          )}
          {yTicks.map((v) =>
            v === 0 ? null : (
              <text key={`ty${v}`} className="plot-tick" x={Math.min(Math.max(sx(0) - 6, 30), width - 4)} y={sy(v) + 4} textAnchor="end">
                {label(v)}
              </text>
            ),
          )}
          {points.map((p, i) => (
            <circle
              key={`p${i}`}
              className={`plot-point kind-${p.kind}${i === selected ? ' selected' : ''}`}
              cx={sx(p.x)}
              cy={sy(p.y)}
              r={i === selected ? 7 : 5}
              onPointerEnter={() => setHovered(i)}
              onPointerLeave={() => setHovered(null)}
              onClick={() => onSelect(i === selected ? null : i)}
            />
          ))}
        </svg>
        {shown !== null && points[shown] && (
          <div className="plot-tooltip" style={{ left: sx(points[shown].x) + 10, top: sy(points[shown].y) - 34 }}>
            <Katex latex={points[shown].latex} />
          </div>
        )}
      </div>
      <div className="plot-footer">
        <span className="plot-hint">{t('plot.hint')}</span>
        <button className="copy-btn" onClick={reset}>
          {t('plot.reset')}
        </button>
      </div>
    </div>
  );
}
