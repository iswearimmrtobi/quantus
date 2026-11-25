import { useState, type PointerEvent as ReactPointerEvent, type ReactNode } from 'react';
import { onPath, pointKey, snap, type Path, type Placeable, type Point, type Rect } from './geometry';
import type { Drag } from './editing';
import { leaderOf, type CanvasLabel, type Placement } from './labels';

/** The radius of a pin's handle and a wire end's, drawing units. */
export const PIN_RADIUS = 5;

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
    return <polyline className="wire-ghost" points={polylinePoints(drag.path)} />;
  }
  return null;
}

/** A path as the `points` of a `<polyline>`. */
export const polylinePoints = (path: Path) => path.map((p) => `${p.x},${p.y}`).join(' ');

interface WireProps {
  /** From its start through its corners to its end. */
  points: Path;
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
export function WirePart({ points, className, lineClassName, joins, connected, onPointerDown, onEnd, onTerminal }: WireProps) {
  /** The grid point on the wire under the pointer, where a new wire can start. */
  const [junction, setJunction] = useState<Point | null>(null);
  const [a, b] = [points[0], points[points.length - 1]];
  const line = polylinePoints(points);
  const dot = (point: Point) => (connected.has(pointKey(point)) ? 'end connected' : 'end');
  const findJunction = (event: ReactPointerEvent<SVGElement>) => {
    const matrix = (event.currentTarget as SVGGraphicsElement).getScreenCTM()?.inverse();
    if (!matrix || !joins) return setJunction(null);
    const at = new DOMPoint(event.clientX, event.clientY).matrixTransform(matrix);
    const point = { x: snap(at.x), y: snap(at.y) };
    const near = Math.hypot(at.x - point.x, at.y - point.y) <= 4 && onPath(points, point);
    setJunction((old) => (near ? (old && pointKey(old) === pointKey(point) ? old : point) : null));
  };
  return (
    <g className={className} onPointerMove={findJunction} onPointerLeave={() => setJunction(null)}>
      <polyline className="part-hit" points={line} onPointerDown={onPointerDown} />
      <polyline className={lineClassName ? `wire ${lineClassName}` : 'wire'} points={line} pointerEvents="none" />
      <circle className={dot(a)} cx={a.x} cy={a.y} r={PIN_RADIUS} onPointerDown={(event) => onEnd(event, 'a')} />
      <circle className={dot(b)} cx={b.x} cy={b.y} r={PIN_RADIUS} onPointerDown={(event) => onEnd(event, 'b')} />
      {junction && (
        <circle
          className="end"
          cx={junction.x}
          cy={junction.y}
          r={PIN_RADIUS}
          onPointerDown={(event) => {
            setJunction(null);
            onTerminal(event, junction);
          }}
        />
      )}
    </g>
  );
}

interface PlacedPartProps {
  element: Placeable;
  className: string;
  /** The part in its own coordinates. */
  symbol: ReactNode;
  /** Where it is grabbed, in its own coordinates. */
  hit: Rect;
  pins: Point[];
  /** Drawn over the part at its place, unturned. */
  decoration?: ReactNode;
  /** The points where pins meet; their handles are hidden until hovered. */
  connected: Set<string>;
  onPointerDown(event: ReactPointerEvent): void;
  onPin(event: ReactPointerEvent, point: Point): void;
}

/** A part at its place, turned: its symbol, where it is grabbed, and its pins to draw wires from. */
export function PlacedPart({ element, className, symbol, hit, pins, decoration, connected, onPointerDown, onPin }: PlacedPartProps) {
  return (
    <g className={className}>
      <g transform={`translate(${element.x} ${element.y}) rotate(${element.rotation * 90})`}>
        <g className="symbol">{symbol}</g>
        <rect className="part-hit" x={hit.x} y={hit.y} width={hit.w} height={hit.h} onPointerDown={onPointerDown} />
      </g>
      {pins.map((point, k) => (
        <circle
          key={k}
          className={connected.has(pointKey(point)) ? 'terminal connected' : 'terminal'}
          cx={point.x}
          cy={point.y}
          r={PIN_RADIUS}
          onPointerDown={(event) => onPin(event, point)}
        />
      ))}
      {decoration}
    </g>
  );
}

/** The filled dots where three or more lines meet. */
export function JunctionDots({ dots }: { dots: Point[] }) {
  return (
    <>
      {dots.map((point) => (
        <circle key={pointKey(point)} className="junction-dot" cx={point.x} cy={point.y} r="3.5" />
      ))}
    </>
  );
}

/** The dashed lines to the labels that had to move away from what they belong to. */
export function LeaderLines({ labels, placements }: { labels: CanvasLabel[]; placements: Map<string, Placement> }) {
  return (
    <>
      {labels.map((label) => {
        const placement = placements.get(label.id);
        if (!placement?.displaced) return null;
        const [from, to] = leaderOf(label, placement);
        return <line key={label.id} className="label-leader" x1={from.x} y1={from.y} x2={to.x} y2={to.y} />;
      })}
    </>
  );
}
