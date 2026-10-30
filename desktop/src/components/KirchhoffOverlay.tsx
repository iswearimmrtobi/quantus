import type { Kirchhoff } from '../api';
import { GRID, terminals, type EditorElement, type Point } from '../circuit';
import { useT } from '../i18n';
import type { Rect } from '../labelLayout';
import type { ExtraLabel } from './CircuitEditor';
import { Katex } from './Katex';

/** What the pointer rests on, in the overlay or the working: its parts are highlighted. */
export type Hover = { kind: 'loop' | 'junction'; index: number } | null;

/** The radius of a loop's circular arrow. */
const LOOP_R = 14;
/** How far a current arrow runs beside its part, and how long it is. */
const ARROW_OFFSET = 22;
const ARROW_HALF = 14;
/** A voltage arrow runs on the other side, where the part's label is, and longer; closer to a
 * resistor than to a source's circle. */
const VOLTAGE_OFFSET = 14;
const VOLTAGE_HALF = 18;

interface Arrow {
  branch: number;
  lead: string;
  from: Point;
  to: Point;
  /** The side its label goes to, away from the part. */
  side: 'above' | 'left';
}

/** A voltage arrow beside a part, in the direction its U is counted. */
interface VoltageArrow {
  id: string;
  from: Point;
  to: Point;
}

/** A drawn junction: where its dot goes. */
export interface JunctionMark {
  index: number;
  point: Point;
}

/**
 * An arrow beside a part from terminal `a` to `b`, or back. A horizontal part has its label below,
 * a vertical one on the right; `outside` puts the arrow on the other side, away from the label.
 */
function beside(element: EditorElement, forward: boolean, outside: boolean, offset: number, half: number) {
  const { a, b } = terminals(element) as Record<'a' | 'b', Point>;
  const [p, q] = forward ? [a, b] : [b, a];
  const length = Math.hypot(q.x - p.x, q.y - p.y) || 1;
  const u = { x: (q.x - p.x) / length, y: (q.y - p.y) / length };
  const horizontal = Math.abs(u.x) >= Math.abs(u.y);
  const away = outside ? -offset : offset;
  const shift = horizontal ? { x: 0, y: away } : { x: away, y: 0 };
  const centre = { x: (a.x + b.x) / 2 + shift.x, y: (a.y + b.y) / 2 + shift.y };
  return {
    from: { x: centre.x - u.x * half, y: centre.y - u.y * half },
    to: { x: centre.x + u.x * half, y: centre.y + u.y * half },
    horizontal,
  };
}

/**
 * The current arrow of each branch, in the direction of the current, above or left of a part:
 * its first resistor, where the arrow points from `+` to `−` as the current runs outside the
 * source, or else its lead part. A click flips the branch through its lead either way.
 */
export function currentArrows(solution: Kirchhoff, elements: EditorElement[]): Arrow[] {
  const arrows: Arrow[] = [];
  solution.branches.forEach((branch, index) => {
    const parts = branch.members.filter((m) => !m.internal);
    const member =
      parts.find((m) => elements.find((e) => e.id === m.id)?.kind === 'resistor') ?? parts.find((m) => m.id === branch.lead);
    const element = member && elements.find((e) => e.id === member.id);
    // A current source's own conductance is a branch with no part to draw it on.
    if (!member || !element) return;
    const { from, to, horizontal } = beside(element, member.forward, true, ARROW_OFFSET, ARROW_HALF);
    arrows.push({ branch: index, lead: branch.lead, from, to, side: horizontal ? 'above' : 'left' });
  });
  return arrows;
}

/**
 * The voltage arrow of each part, on its label's side. A resistor's runs with its branch current,
 * a voltage source's from `+` to `−`, a current source's against the current it drives.
 */
export function voltageArrows(solution: Kirchhoff, elements: EditorElement[]): VoltageArrow[] {
  const forward = new Map(solution.branches.flatMap((b) => b.members.filter((m) => !m.internal).map((m) => [m.id, m.forward])));
  const arrows: VoltageArrow[] = [];
  for (const { id } of solution.elements) {
    const element = elements.find((e) => e.id === id);
    if (!element) continue;
    const direction =
      element.kind === 'voltageSource' ? true : element.kind === 'currentSource' ? false : forward.get(id);
    if (direction === undefined) continue;
    const offset = element.kind === 'resistor' ? VOLTAGE_OFFSET : ARROW_OFFSET;
    const { from, to } = beside(element, direction, false, offset, VOLTAGE_HALF);
    arrows.push({ id, from, to });
  }
  return arrows;
}

const loopCentre = (anchor: Point): Point => ({ x: anchor.x * GRID, y: anchor.y * GRID });

/** The labels of the overlay, laid out by the editor with the part labels. */
export function overlayLabels(solution: Kirchhoff, junctions: JunctionMark[], arrows: Arrow[]): ExtraLabel[] {
  const labels: ExtraLabel[] = junctions.map(({ index, point }) => ({
    id: `junction-${index}`,
    className: 'kirchhoff-label kirchhoff-junction-name',
    anchor: point,
    gap: 6,
    corner: true,
    sides: ['above', 'below', 'right', 'left'],
    content: <Katex latex={solution.junctions[index].name} />,
  }));
  for (const arrow of arrows) {
    labels.push({
      id: `current-${arrow.branch}`,
      className: 'kirchhoff-label kirchhoff-current-name',
      anchor: { x: (arrow.from.x + arrow.to.x) / 2, y: (arrow.from.y + arrow.to.y) / 2 },
      gap: 4,
      sides: arrow.side === 'above' ? ['above', 'right', 'left', 'below'] : ['left', 'above', 'below', 'right'],
      content: <Katex latex={solution.branches[arrow.branch].current} />,
    });
  }
  return labels;
}

/** What the overlay draws, so the labels keep clear of it. */
export function overlayReserved(solution: Kirchhoff, arrows: { from: Point; to: Point }[]): Rect[] {
  const rects: Rect[] = solution.loops.map(({ anchor }) => {
    const c = loopCentre(anchor);
    const r = LOOP_R + 4;
    return { x: c.x - r, y: c.y - r, w: 2 * r, h: 2 * r };
  });
  for (const { from, to } of arrows) {
    rects.push({
      x: Math.min(from.x, to.x) - 5,
      y: Math.min(from.y, to.y) - 5,
      w: Math.abs(to.x - from.x) + 10,
      h: Math.abs(to.y - from.y) + 10,
    });
  }
  return rects;
}

/** A filled arrowhead with its tip at `tip`, pointing along `u`. */
function head(tip: Point, u: Point, size = 7): string {
  const back = { x: tip.x - u.x * size, y: tip.y - u.y * size };
  const side = { x: -u.y * size * 0.55, y: u.x * size * 0.55 };
  return `${tip.x},${tip.y} ${back.x + side.x},${back.y + side.y} ${back.x - side.x},${back.y - side.y}`;
}

/** An open arrowhead, two strokes meeting at `tip`, pointing along `u`. */
function openHead(tip: Point, u: Point, size = 6): string {
  const back = { x: tip.x - u.x * size, y: tip.y - u.y * size };
  const side = { x: -u.y * size * 0.6, y: u.x * size * 0.6 };
  return `${back.x + side.x},${back.y + side.y} ${tip.x},${tip.y} ${back.x - side.x},${back.y - side.y}`;
}

/** `L_{1}` as SVG text with a subscript; KaTeX does not draw inside SVG in WebKitGTK. */
function Name({ latex, at }: { latex: string; at: Point }) {
  const match = /^(.+?)_\{(.+)\}$/.exec(latex);
  return (
    <text className="kirchhoff-loop-name" x={at.x} y={at.y} textAnchor="middle" dominantBaseline="central">
      {match ? match[1] : latex}
      {match && (
        <tspan dy="3" fontSize="8">
          {match[2]}
        </tspan>
      )}
    </text>
  );
}

interface Props {
  solution: Kirchhoff;
  junctions: JunctionMark[];
  arrows: Arrow[];
  /** Empty while the parts don't show their voltage. */
  voltages: VoltageArrow[];
  hover: Hover;
  onHover(hover: Hover): void;
  /** Flips the reference direction of the branch the part leads. */
  onFlip(lead: string): void;
}

/** Junction dots, current arrows that flip on a click, voltage arrows, and a clockwise arrow in every loop. */
export function KirchhoffOverlay({ solution, junctions, arrows, voltages, hover, onHover, onFlip }: Props) {
  const { t } = useT();
  return (
    <g className="kirchhoff-overlay">
      {solution.loops.map((loop, index) => {
        const c = loopCentre(loop.anchor);
        // Clockwise on the screen (y points down) from just right of the top round to just left of it.
        const angle = (degrees: number) => (degrees * Math.PI) / 180;
        const [start, end] = [angle(-60), angle(240)];
        const p = { x: c.x + LOOP_R * Math.cos(start), y: c.y + LOOP_R * Math.sin(start) };
        const q = { x: c.x + LOOP_R * Math.cos(end), y: c.y + LOOP_R * Math.sin(end) };
        const tangent = { x: -Math.sin(end), y: Math.cos(end) };
        const active = hover?.kind === 'loop' && hover.index === index;
        return (
          <g
            key={`loop-${index}`}
            className={active ? 'kirchhoff-loop active' : 'kirchhoff-loop'}
            onPointerEnter={() => onHover({ kind: 'loop', index })}
            onPointerLeave={() => onHover(null)}
          >
            <circle className="kirchhoff-hit" cx={c.x} cy={c.y} r={LOOP_R + 6} />
            <path d={`M${p.x} ${p.y}A${LOOP_R} ${LOOP_R} 0 1 1 ${q.x} ${q.y}`} />
            <polygon points={head({ x: q.x + tangent.x * 3, y: q.y + tangent.y * 3 }, tangent, 6)} />
            <Name latex={loop.name} at={c} />
          </g>
        );
      })}

      {voltages.map(({ id, from, to }) => {
        const length = Math.hypot(to.x - from.x, to.y - from.y) || 1;
        const u = { x: (to.x - from.x) / length, y: (to.y - from.y) / length };
        return (
          <g key={`voltage-${id}`} className="kirchhoff-voltage">
            <line x1={from.x} y1={from.y} x2={to.x} y2={to.y} />
            <polyline points={openHead(to, u)} />
          </g>
        );
      })}

      {arrows.map((arrow) => {
        const length = Math.hypot(arrow.to.x - arrow.from.x, arrow.to.y - arrow.from.y) || 1;
        const u = { x: (arrow.to.x - arrow.from.x) / length, y: (arrow.to.y - arrow.from.y) / length };
        return (
          <g
            key={`current-${arrow.branch}`}
            className="kirchhoff-current"
            onPointerDown={(event) => event.stopPropagation()}
            onClick={() => onFlip(arrow.lead)}
          >
            <title>{t('kirchhoff.flip')}</title>
            <line className="kirchhoff-hit" x1={arrow.from.x} y1={arrow.from.y} x2={arrow.to.x} y2={arrow.to.y} />
            <line x1={arrow.from.x} y1={arrow.from.y} x2={arrow.to.x - u.x * 5} y2={arrow.to.y - u.y * 5} />
            <polygon points={head(arrow.to, u)} />
          </g>
        );
      })}

      {junctions.map(({ index, point }) => (
        <circle
          key={`junction-${index}`}
          className={hover?.kind === 'junction' && hover.index === index ? 'kirchhoff-junction active' : 'kirchhoff-junction'}
          cx={point.x}
          cy={point.y}
          r={4}
        />
      ))}
    </g>
  );
}
