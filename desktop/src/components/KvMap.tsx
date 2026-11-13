import { useState, type KeyboardEvent, type ReactElement } from 'react';
import type { Kmap, LogicScene, LogicValue } from '../api';
import { cycle, editColumn, type Cell, type Grid, type Shown } from '../logic';
import { useT } from '../i18n';
import { Katex } from './Katex';

interface Props {
  scene: LogicScene;
  grid: Grid;
  shown: Shown;
  /** Whether 0s are written or left empty. */
  zeros: boolean;
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
 * The KV diagram of one output (§6.3): its maps with the variables' bars, each cell with its value
 * and the row number in its corner. Clicking a cell cycles it like the table's and makes the
 * table the output's source.
 */
export function KvMap({ scene, grid, shown, zeros, onChange }: Props) {
  const { t } = useT();
  const [numbers, setNumbers] = useNumbers();
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

  return (
    <section className="section-card kv-card">
      <div className="kv-head">
        <span className="section-title">{t('logic.kv.title')}</span>
        <button className={numbers ? 'form-chip active' : 'form-chip'} title={t('logic.kv.numbers.hint')} onClick={() => setNumbers(!numbers)}>
          i
        </button>
      </div>
      <div className="kv-scroll">
        <div className="kv-diagram" style={{ width: at.width, height: at.height }}>
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
              >
                {numbers && <span className="kv-number">{cell.index}</span>}
                {text(shownValue)}
              </button>
            );
          })}
        </div>
      </div>
    </section>
  );
}
