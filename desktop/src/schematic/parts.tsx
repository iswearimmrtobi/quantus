import { useState, type PointerEvent as ReactPointerEvent } from 'react';
import { corner, inside, pointKey, snap, type Point } from './geometry';
import type { Drag } from './editing';

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
    const near = Math.hypot(at.x - point.x, at.y - point.y) <= 4 && inside(a, b, point);
    setJunction((old) => (near ? (old && pointKey(old) === pointKey(point) ? old : point) : null));
  };
  return (
    <g className={className} onPointerMove={findJunction} onPointerLeave={() => setJunction(null)}>
      <line className="part-hit" x1={a.x} y1={a.y} x2={b.x} y2={b.y} onPointerDown={onPointerDown} />
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
