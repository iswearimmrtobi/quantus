import { useState, type KeyboardEvent, type ReactElement } from 'react';
import type { Kmap, KvBlockResult, LogicScene, LogicValue } from '../api';
import { cycle, editColumn, type Cell, type Grid, type Shown, type TermHover } from '../logic';
import { useT } from '../i18n';
import { Katex } from './Katex';

interface Props {
  scene: LogicScene;
  grid: Grid;
  shown: Shown;
  /** Whether 0s are written or left empty. */
  zeros: boolean;
  /** The term hovered here or in the results; its block stands out. */
  hover: TermHover | null;
  onHover(hover: TermHover | null): void;
  onChange(scene: LogicScene): void;
}

/** The side of a cell, px. */
const CELL = 36;
/** The room around a map for its bars, px. */
const MARGIN = 30;
/** The room around the grid of maps for the bars of the map variables, px. */
const OUTER = 30;
/** Between two maps, px. */
const GAP = 14;
/** How far a bar lies from the edge it labels, and its label beyond it, px. */
const BAR = 8;
const LABEL = 20;

const NUMBERS_KEY = 'quantus.logic.kvNumbers';

/** Whether the row numbers show in the cells' corners, remembered per browser. */
function useNumbers(): [boolean, (on: boolean) => void] {
  const [on, setOn] = useState(() => {
    try {
      return localStorage.getItem(NUMBERS_KEY) !== 'off';
    } catch {
      return true;
    }
  });
  const set = (value: boolean) => {
    setOn(value);
    try {
      localStorage.setItem(NUMBERS_KEY, value ? 'on' : 'off');
    } catch {
      // Not remembered; the view still switches.
    }
  };
  return [on, set];
}

/** Where the maps and their cells lie, px. */
export interface Geometry {
  width: number;
  height: number;
  mapWidth: number;
  mapHeight: number;
  /** The top left corner of cell (0, 0) of map `map`. */
  origin(map: number): { x: number; y: number };
}

export function geometry(kmap: Kmap): Geometry {
  const columns = Math.max(...kmap.maps.map((m) => m.x)) + 1;
  const rows = Math.max(...kmap.maps.map((m) => m.y)) + 1;
  const left = kmap.mapVars.length > 1 ? OUTER : 0;
  const top = kmap.mapVars.length > 0 ? OUTER : 0;
  const mapWidth = kmap.columns * CELL + 2 * MARGIN;
  const mapHeight = kmap.rows * CELL + 2 * MARGIN;
  const place = new Map(kmap.maps.map((m) => [m.map, m]));
  return {
    width: left + columns * mapWidth + (columns - 1) * GAP,
    height: top + rows * mapHeight + (rows - 1) * GAP,
    mapWidth,
    mapHeight,
    origin(map) {
      const m = place.get(map) ?? { x: 0, y: 0 };
      return { x: left + m.x * (mapWidth + GAP) + MARGIN, y: top + m.y * (mapHeight + GAP) + MARGIN };
    },
  };
}

/** How far the blocks' edges lie inside their cells, the first block's and each further one's, px. */
const INSET = 3;
const INSET_STEP = 3;
/** How far an open side reaches past the map's edge, px. */
const OVERHANG = 7;

/** A block as drawn: its pieces, its colour (an index into the palette) and how it stands out. */
export interface DrawnBlock {
  key: string;
  pieces: KvBlockResult['pieces'];
  colour: number;
  /** Nested blocks lie further inside their cells. */
  depth: number;
  className?: string;
  /** A label in the first piece's corner. */
  label?: string;
}

/** The corner radius of a block, px. */
const RADIUS = 9;

/**
 * A piece's outline as SVG paths: `fill` closed, `stroke` without the open sides; the corners next
 * to an open side are square.
 */
export function piecePaths(x: number, y: number, w: number, h: number, open: { left: boolean; right: boolean; top: boolean; bottom: boolean }) {
  // The sides in order, each from its corner to the next: top, right, bottom, left.
  const sides = [open.top, open.right, open.bottom, open.left];
  const corners = [
    [x, y],
    [x + w, y],
    [x + w, y + h],
    [x, y + h],
  ];
  const r = Math.min(RADIUS, w / 2, h / 2);
  // A corner is round when neither side at it is open; corner k joins side k - 1 and side k.
  const radius = corners.map((_, k) => (sides[(k + 3) % 4] || sides[k] ? 0 : r));
  // The unit direction of side k.
  const dir = [
    [1, 0],
    [0, 1],
    [-1, 0],
    [0, -1],
  ];
  const at = (k: number, along: number) => [corners[k][0] + dir[k][0] * along, corners[k][1] + dir[k][1] * along];
  const len = (k: number) => (k % 2 === 0 ? w : h);
  const p = (point: number[]) => `${point[0].toFixed(1)} ${point[1].toFixed(1)}`;
  const side = (k: number) => {
    const next = (k + 1) % 4;
    const from = at(k, radius[k]);
    const to = at(k, len(k) - radius[next]);
    const arc = radius[next] ? ` A ${radius[next]} ${radius[next]} 0 0 1 ${p(at(next, radius[next]))}` : '';
    return { from, line: `L ${p(to)}${arc}` };
  };
  const fill = `M ${p(side(0).from)} ${[0, 1, 2, 3].map((k) => side(k).line).join(' ')} Z`;
  const closed = sides.every((o) => !o);
  const stroke = closed
    ? fill
    : [0, 1, 2, 3]
        .filter((k) => !sides[k])
        .map((k) => `M ${p(side(k).from)} ${side(k).line}`)
        .join(' ');
  return { fill, stroke };
}

/** The blocks over the cells (§6.3), in one SVG: rounded rectangles, a piece open towards each edge it wraps over. */
function Blocks({ blocks, at }: { blocks: DrawnBlock[]; at: Geometry }) {
  return (
    <svg className="kv-blocks" width={at.width} height={at.height} aria-hidden>
      {blocks.map((block) => (
        <g key={block.key} className={['kv-block', block.className ?? ''].join(' ')} style={{ ['--block' as string]: `var(--geometry-${block.colour % 8})` }}>
          {block.pieces.map((piece, p) => {
            const { x, y } = at.origin(piece.map);
            const inset = INSET + block.depth * INSET_STEP;
            const left = x + piece.col * CELL + (piece.open.left ? -OVERHANG : inset);
            const top = y + piece.row * CELL + (piece.open.top ? -OVERHANG : inset);
            const right = x + (piece.col + piece.width) * CELL + (piece.open.right ? OVERHANG : -inset);
            const bottom = y + (piece.row + piece.height) * CELL + (piece.open.bottom ? OVERHANG : -inset);
            const { fill, stroke } = piecePaths(left, top, right - left, bottom - top, piece.open);
            return (
              <g key={p}>
                <path className="kv-block-fill" d={fill} />
                <path className="kv-block-line" d={stroke} />
                {p === 0 && block.label && (
                  <g className="kv-block-label">
                    <circle cx={left} cy={top} r={8} />
                    <text x={left} y={top} dy="0.35em">
                      {block.label}
                    </text>
                  </g>
                )}
              </g>
            );
          })}
        </g>
      ))}
    </svg>
  );
}

/** The bars of the variables (§6.3): a line over the rows or columns where it is 1, its name beside it. */
function Bars({ kmap, at }: { kmap: Kmap; at: Geometry }) {
  const parts: ReactElement[] = [];
  const line = (key: string, x: number, y: number, w: number, h: number) => <div key={key} className="kv-bar" style={{ left: x, top: y, width: w, height: h }} />;
  const label = (key: string, latex: string, x: number, y: number) => (
    <span key={key} className="kv-bar-label" style={{ left: x, top: y }}>
      <Katex latex={latex} />
    </span>
  );
  kmap.bars.forEach((bar, b) => {
    if (bar.maps) {
      // Along the grid of maps: the first map variable over the right column, the second left of the bottom row.
      const left = kmap.mapVars.length > 1 ? OUTER : 0;
      const top = OUTER;
      if (bar.edge === 'top') {
        const x = left + bar.start * (at.mapWidth + GAP) + MARGIN;
        const w = at.mapWidth - 2 * MARGIN;
        parts.push(line(`${b}`, x, top - BAR, w, 0), label(`${b}l`, bar.latex, x + w / 2, top - LABEL));
      } else {
        const y = top + bar.start * (at.mapHeight + GAP) + MARGIN;
        const h = at.mapHeight - 2 * MARGIN;
        parts.push(line(`${b}`, left - BAR, y, 0, h), label(`${b}l`, bar.latex, left - LABEL, y + h / 2));
      }
      return;
    }
    for (const { map } of kmap.maps) {
      const { x, y } = at.origin(map);
      const from = bar.start * CELL;
      const len = bar.len * CELL;
      const key = `${b}-${map}`;
      const [w, h] = [kmap.columns * CELL, kmap.rows * CELL];
      if (bar.edge === 'top') parts.push(line(key, x + from, y - BAR, len, 0), label(`${key}l`, bar.latex, x + from + len / 2, y - LABEL));
      else if (bar.edge === 'bottom') parts.push(line(key, x + from, y + h + BAR, len, 0), label(`${key}l`, bar.latex, x + from + len / 2, y + h + LABEL));
      else if (bar.edge === 'left') parts.push(line(key, x - BAR, y + from, 0, len), label(`${key}l`, bar.latex, x - LABEL, y + from + len / 2));
      else parts.push(line(key, x + w + BAR, y + from, 0, len), label(`${key}l`, bar.latex, x + w + LABEL, y + from + len / 2));
    }
  });
  return <>{parts}</>;
}

/**
 * How deep each block lies: one more than the deepest earlier block it shares a cell with, so
 * that blocks over the same cells show inside each other.
 */
export function depths(blocks: { cells: number[] }[]): number[] {
  const found: number[] = [];
  blocks.forEach((block, b) => {
    const below = found.filter((_, o) => blocks[o].cells.some((cell) => block.cells.includes(cell)));
    found[b] = below.length ? Math.max(...below) + 1 : 0;
  });
  return found.map((d) => d % 4);
}

/**
 * The KV diagram of one output (§6.3): its maps with the variables' bars, each cell with its value
 * and the row number in its corner, and the blocks of the chosen minimal DNF over the 1s (or of
 * the KNF over the 0s) in colour. Clicking a cell cycles it like the table's and makes the table
 * the output's source; hovering it, or a term in the legend or the results, singles out a block.
 */
export function KvMap({ scene, grid, shown, zeros, hover, onHover, onChange }: Props) {
  const { t } = useT();
  const [numbers, setNumbers] = useNumbers();
  /** The blocks shown: over the 1s for the minimal DNF, over the 0s for the minimal KNF. */
  const [form, setForm] = useState<'dnf' | 'knf'>('dnf');
  const { result, index } = shown;
  const kmap = result.kmap;
  if (!kmap) {
    return (
      <section className="section-card kv-card">
        <span className="section-title">{t('logic.kv.title')}</span>
        <p className="analysis-empty">{result.table.length ? t('logic.kv.none') : t('logic.result.none')}</p>
      </section>
    );
  }
  const at = geometry(kmap);
  const column = grid.columns.find((c) => (index === null ? c.index === null && c.name === result.name : c.index === index));
  const value = (row: number, fallback: LogicValue): Cell => column?.values[row] ?? (column ? null : fallback);
  const vars = [...kmap.mapVars, ...kmap.colVars, ...kmap.rowVars].join('');
  const set = (row: number, next: LogicValue) => {
    if (index !== null) onChange(editColumn(scene, grid, index, (values) => values.map((v, r) => (r === row ? next : v))));
  };
  const onKey = (event: KeyboardEvent, row: number) => {
    if (event.key === '0' || event.key === '1' || event.key === '-') {
      event.preventDefault();
      set(row, event.key);
    }
  };
  const text = (cell: Cell) => (cell === null ? '' : cell === '-' ? '–' : cell === '0' && !zeros ? '' : cell);

  const blocks = form === 'dnf' ? kmap.blocks : kmap.knfBlocks;
  const hovered = hover && hover.output === result.name && hover.form === form ? hover.term : null;
  const nested = depths(blocks);
  const drawn: DrawnBlock[] = blocks.map((block, b) => ({
    key: `${form}-${block.term}`,
    pieces: block.pieces,
    colour: block.term,
    depth: nested[b],
    className: hovered === null ? undefined : hovered === block.term ? 'active' : 'faded',
  }));
  /** Hovering a cell picks the smallest of the blocks over it. */
  const hoverCell = (row: number) => {
    const over = blocks.filter((block) => block.cells.includes(row)).sort((a, b) => a.cells.length - b.cells.length)[0];
    onHover(over ? { output: result.name, form, term: over.term } : null);
  };

  return (
    <section className="section-card kv-card">
      <div className="kv-head">
        <span className="section-title">{t('logic.kv.title')}</span>
        <button className={form === 'dnf' ? 'form-chip active' : 'form-chip'} title={t('logic.kv.dnf.hint')} onClick={() => setForm('dnf')}>
          {t('logic.kv.dnf')}
        </button>
        <button className={form === 'knf' ? 'form-chip active' : 'form-chip'} title={t('logic.kv.knf.hint')} onClick={() => setForm('knf')}>
          {t('logic.kv.knf')}
        </button>
        <button className={numbers ? 'form-chip active' : 'form-chip'} title={t('logic.kv.numbers.hint')} onClick={() => setNumbers(!numbers)}>
          i
        </button>
      </div>
      <div className="kv-scroll">
        <div className="kv-diagram" style={{ width: at.width, height: at.height }} onMouseLeave={() => onHover(null)}>
          <Bars kmap={kmap} at={at} />
          {kmap.cells.map((cell) => {
            const { x, y } = at.origin(cell.map);
            const shownValue = value(cell.index, cell.value);
            return (
              <button
                key={cell.index}
                className={shownValue === null ? 'kv-cell empty' : 'kv-cell'}
                style={{ left: x + cell.col * CELL, top: y + cell.row * CELL, width: CELL + 1, height: CELL + 1 }}
                title={t('logic.kv.cell', { index: cell.index, vars, code: cell.code })}
                disabled={index === null}
                onClick={() => set(cell.index, cycle(shownValue))}
                onKeyDown={(event) => onKey(event, cell.index)}
                onMouseEnter={() => hoverCell(cell.index)}
                onFocus={() => hoverCell(cell.index)}
              >
                {numbers && <span className="kv-number">{cell.index}</span>}
                {text(shownValue)}
              </button>
            );
          })}
          <Blocks blocks={drawn} at={at} />
        </div>
      </div>
      {blocks.length > 0 && (
        <div className="kv-legend">
          {blocks.map((block) => (
            <span
              key={block.term}
              className={hovered === block.term ? 'kv-term active' : 'kv-term'}
              style={{ ['--block' as string]: `var(--geometry-${block.term % 8})` }}
              onMouseEnter={() => onHover({ output: result.name, form, term: block.term })}
              onMouseLeave={() => onHover(null)}
            >
              <Katex latex={block.latex} />
            </span>
          ))}
        </div>
      )}
    </section>
  );
}
