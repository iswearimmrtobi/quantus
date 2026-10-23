import { useEffect, useMemo, useRef, useState, type PointerEvent as ReactPointerEvent, type ReactNode } from 'react';
import { api, ApiError, type CircuitResponse, type CircuitValue } from '../api';
import {
  FIELDS,
  GRID,
  HALF,
  KIND_LABELS,
  MATERIALS,
  newElement,
  numberNodes,
  pointKey,
  snap,
  terminals,
  toRequest,
  type EditorElement,
  type EditorKind,
  type Point,
  type Terminal,
} from '../circuit';
import { Katex } from './Katex';

interface Props {
  elements: EditorElement[];
  onChange(next: EditorElement[]): void;
  /** Extra controls for the palette column (load and save). */
  toolbar?: ReactNode;
}

type Drag =
  | { type: 'move'; id: string; from: Point; origin: EditorElement }
  | { type: 'end'; id: string; end: 'a' | 'b' }
  | { type: 'draw'; from: Point; to: Point };

const PALETTE: EditorKind[] = [
  'voltageSource',
  'currentSource',
  'resistor',
  'potentiometer',
  'powerLoad',
  'wire',
  'ground',
];

export function CircuitEditor({ elements, onChange, toolbar }: Props) {
  const [selected, setSelected] = useState<string | null>(null);
  const [drag, setDrag] = useState<Drag | null>(null);
  const [response, setResponse] = useState<CircuitResponse | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const canvas = useRef<SVGSVGElement>(null);

  const nodes = useMemo(() => numberNodes(elements), [elements]);
  const request = useMemo(() => toRequest(elements, nodes), [elements, nodes]);

  // Every change recalculates; the engine runs in the app, so a short debounce is enough to keep
  // dragging smooth. A slow answer to an older circuit must not overwrite a newer one.
  useEffect(() => {
    let current = true;
    const timer = window.setTimeout(() => {
      if (request.elements.length === 0) {
        setResponse(null);
        setFailure(null);
        return;
      }
      api.solveCircuit(request).then(
        (solved) => {
          if (!current) return;
          setResponse(solved);
          setFailure(null);
        },
        (error) => {
          if (!current) return;
          setResponse(null);
          setFailure(error instanceof ApiError ? error.message : String(error));
        },
      );
    }, 60);
    return () => {
      current = false;
      window.clearTimeout(timer);
    };
  }, [request]);

  const results = useMemo(() => new Map(response?.elements.map((e) => [e.id, e])), [response]);
  const voltages = useMemo(() => new Map(response?.nodes.map((n) => [n.id, n.voltage])), [response]);

  const update = (id: string, change: Partial<EditorElement>) =>
    onChange(elements.map((e) => (e.id === id ? { ...e, ...change } : e)));

  const add = (kind: EditorKind, at?: Point) => {
    // Without a drop position the part goes to a free spot, one row below the last one.
    const spot = at ?? { x: 160 + (elements.length % 4) * 3 * GRID, y: 120 + Math.floor(elements.length / 4) * 3 * GRID };
    const element = newElement(elements, kind, spot);
    onChange([...elements, element]);
    setSelected(element.id);
  };

  const remove = (id: string) => {
    onChange(elements.filter((e) => e.id !== id));
    setSelected(null);
  };

  const rotateSelected = () => {
    const element = elements.find((e) => e.id === selected);
    if (!element) return;
    if (element.kind === 'wire') {
      // A wire turns about its start point.
      const dx = (element.x2 ?? element.x) - element.x;
      const dy = (element.y2 ?? element.y) - element.y;
      update(element.id, { x2: element.x - dy, y2: element.y + dx });
    } else {
      update(element.id, { rotation: (element.rotation + 1) % 4 });
    }
  };

  const toCanvas = (event: { clientX: number; clientY: number }): Point => {
    const box = canvas.current!.getBoundingClientRect();
    return { x: snap(event.clientX - box.left), y: snap(event.clientY - box.top) };
  };

  const startMove = (event: ReactPointerEvent, element: EditorElement) => {
    event.stopPropagation();
    canvas.current?.setPointerCapture(event.pointerId);
    setSelected(element.id);
    setDrag({ type: 'move', id: element.id, from: toCanvas(event), origin: element });
  };

  const startEnd = (event: ReactPointerEvent, element: EditorElement, end: 'a' | 'b') => {
    event.stopPropagation();
    canvas.current?.setPointerCapture(event.pointerId);
    setSelected(element.id);
    setDrag({ type: 'end', id: element.id, end });
  };

  const startDraw = (event: ReactPointerEvent, from: Point) => {
    event.stopPropagation();
    canvas.current?.setPointerCapture(event.pointerId);
    setDrag({ type: 'draw', from, to: from });
  };

  const onMove = (event: ReactPointerEvent) => {
    if (!drag) return;
    const at = toCanvas(event);
    if (drag.type === 'draw') {
      if (at.x !== drag.to.x || at.y !== drag.to.y) setDrag({ ...drag, to: at });
    } else if (drag.type === 'end') {
      update(drag.id, drag.end === 'a' ? { x: at.x, y: at.y } : { x2: at.x, y2: at.y });
    } else {
      const dx = at.x - drag.from.x;
      const dy = at.y - drag.from.y;
      const o = drag.origin;
      update(
        drag.id,
        o.kind === 'wire'
          ? { x: o.x + dx, y: o.y + dy, x2: (o.x2 ?? o.x) + dx, y2: (o.y2 ?? o.y) + dy }
          : { x: o.x + dx, y: o.y + dy },
      );
    }
  };

  const onUp = () => {
    if (drag?.type === 'draw' && pointKey(drag.from) !== pointKey(drag.to)) {
      const wire = { ...newElement(elements, 'wire', drag.from), x2: drag.to.x, y2: drag.to.y };
      onChange([...elements, wire]);
      setSelected(wire.id);
    }
    setDrag(null);
  };

  const onKeyDown = (event: React.KeyboardEvent) => {
    if (event.key === 'Delete' || event.key === 'Backspace') {
      if (selected) remove(selected);
    } else if (event.key.toLowerCase() === 'r') {
      rotateSelected();
    } else if (event.key === 'Escape') {
      setSelected(null);
    } else {
      return;
    }
    event.preventDefault();
  };

  const chosen = elements.find((e) => e.id === selected) ?? null;

  return (
    <div className="circuit">
      <div className="circuit-palette">
        <div className="palette-title">BAUTEILE</div>
        {PALETTE.map((kind) => (
          <button
            key={kind}
            className="palette-item"
            draggable
            title="Auf die Zeichenfläche ziehen oder klicken"
            onDragStart={(event) => event.dataTransfer.setData('text/plain', kind)}
            onClick={() => add(kind)}
          >
            <svg viewBox="-48 -24 96 48" width="48" height="24">
              <Symbol kind={kind} />
            </svg>
            <span>{KIND_LABELS[kind]}</span>
          </button>
        ))}
        {toolbar}
        <p className="palette-hint">
          Von einer Klemme ziehen zeichnet eine Leitung. <kbd>R</kbd> dreht, <kbd>Entf</kbd> löscht.
        </p>
      </div>

      <div className="circuit-stage">
        <svg
          ref={canvas}
          className="circuit-canvas"
          tabIndex={0}
          onKeyDown={onKeyDown}
          onPointerMove={onMove}
          onPointerUp={onUp}
          onPointerCancel={() => setDrag(null)}
          onPointerDown={() => {
            setSelected(null);
            canvas.current?.focus();
          }}
          onDragOver={(event) => event.preventDefault()}
          onDrop={(event) => {
            event.preventDefault();
            const kind = event.dataTransfer.getData('text/plain') as EditorKind;
            if (PALETTE.includes(kind)) add(kind, toCanvas(event));
          }}
        >
          <defs>
            <pattern id="grid" width={GRID} height={GRID} patternUnits="userSpaceOnUse">
              <circle cx="0" cy="0" r="0.8" className="grid-dot" />
            </pattern>
          </defs>
          <rect width="100%" height="100%" fill="url(#grid)" pointerEvents="none" />

          {elements.map((element) => (
            <Part
              key={element.id}
              element={element}
              selected={element.id === selected}
              onPointerDown={(event) => startMove(event, element)}
              onTerminal={(event, point) => startDraw(event, point)}
              onEnd={(event, end) => startEnd(event, element, end)}
              result={results.get(element.id)}
            />
          ))}

          {drag?.type === 'draw' && (
            <line className="wire-ghost" x1={drag.from.x} y1={drag.from.y} x2={drag.to.x} y2={drag.to.y} />
          )}

          {[...nodes.pointOf].map(([id, point]) => {
            const voltage = voltages.get(id);
            if (!voltage) return null;
            return (
              <g key={`node-${id}`} transform={`translate(${point.x + 6} ${point.y - 8})`} pointerEvents="none">
                <Value label="φ" value={voltage} small />
              </g>
            );
          })}
        </svg>

        {(failure || (elements.length > 0 && !nodes.grounded && nodes.byPoint.size > 0)) && (
          <div className={failure ? 'circuit-note error' : 'circuit-note'}>
            {failure ??
              'Keine Masse gesetzt: Bezugspotential ist der Minuspol der ersten Spannungsquelle.'}
          </div>
        )}
      </div>

      <aside className="circuit-inspector">
        {chosen ? (
          <Inspector
            element={chosen}
            result={results.get(chosen.id)}
            onChange={(change) => update(chosen.id, change)}
            onRemove={() => remove(chosen.id)}
          />
        ) : (
          <p className="panel-empty">
            {elements.length === 0
              ? 'Bauteile aus der Palette auf die Fläche ziehen, dann Klemmen mit Leitungen verbinden.'
              : 'Ein Bauteil wählen, um seine Werte einzugeben.'}
          </p>
        )}
      </aside>
    </div>
  );
}

/** The symbol of a kind in its own coordinates: terminals at (±HALF, 0), the wiper at (0, −HALF). */
function Symbol({ kind }: { kind: EditorKind }) {
  switch (kind) {
    case 'resistor':
      return (
        <>
          <path d={`M${-HALF} 0H-20M20 0H${HALF}`} />
          <rect x="-20" y="-7" width="40" height="14" />
        </>
      );
    case 'potentiometer':
      return (
        <>
          <path d={`M${-HALF} 0H-20M20 0H${HALF}M0 ${-HALF}V-14`} />
          <rect x="-20" y="-7" width="40" height="14" />
          <path d="M-4 -14L0 -7L4 -14Z" />
        </>
      );
    case 'voltageSource':
      return (
        <>
          <path d={`M${-HALF} 0H-14M14 0H${HALF}`} />
          <circle r="14" />
          <path d="M-9 -3v-5M-11.5 -5.5h5M5 -5.5h5" />
        </>
      );
    case 'currentSource':
      return (
        <>
          <path d={`M${-HALF} 0H-14M14 0H${HALF}`} />
          <circle r="14" />
          <path d="M-8 0H8M3 -4L8 0L3 4" />
        </>
      );
    case 'powerLoad':
      return (
        <>
          <path d={`M${-HALF} 0H-14M14 0H${HALF}`} />
          <rect x="-14" y="-14" width="28" height="28" />
          <path d="M-8 -8L8 8" />
        </>
      );
    case 'ground':
      return <path d="M0 0V8M-12 8H12M-8 13H8M-4 18H4" />;
    case 'wire':
      return <path d={`M${-HALF} 0H${HALF}`} />;
  }
}

interface PartProps {
  element: EditorElement;
  selected: boolean;
  result: CircuitResponse['elements'][number] | undefined;
  onPointerDown(event: ReactPointerEvent): void;
  onTerminal(event: ReactPointerEvent, point: Point): void;
  onEnd(event: ReactPointerEvent, end: 'a' | 'b'): void;
}

function Part({ element, selected, result, onPointerDown, onTerminal, onEnd }: PartProps) {
  const places = terminals(element);
  const className = selected ? 'part selected' : 'part';

  if (element.kind === 'wire') {
    const { a, b } = places as Record<'a' | 'b', Point>;
    const middle = { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
    return (
      <g className={className}>
        <line className="hit" x1={a.x} y1={a.y} x2={b.x} y2={b.y} onPointerDown={onPointerDown} />
        <line className="wire" x1={a.x} y1={a.y} x2={b.x} y2={b.y} pointerEvents="none" />
        <circle className="end" cx={a.x} cy={a.y} r="5" onPointerDown={(event) => onEnd(event, 'a')} />
        <circle className="end" cx={b.x} cy={b.y} r="5" onPointerDown={(event) => onEnd(event, 'b')} />
        <Label element={element} at={{ x: middle.x, y: middle.y - 12 }} result={result} anchor="middle" />
      </g>
    );
  }

  const vertical = element.rotation % 2 === 1;
  const labelAt =
    element.kind === 'ground'
      ? { x: element.x + 18, y: element.y + 14 }
      : vertical
        ? { x: element.x + 24, y: element.y - 8 }
        : { x: element.x - HALF, y: element.y + 26 };

  return (
    <g className={className}>
      <g transform={`translate(${element.x} ${element.y}) rotate(${element.rotation * 90})`}>
        <g className="symbol">
          <Symbol kind={element.kind} />
        </g>
        <rect className="hit" x={-HALF + 8} y="-16" width={2 * HALF - 16} height="32" onPointerDown={onPointerDown} />
      </g>
      {(Object.entries(places) as [Terminal, Point][]).map(([name, point]) => (
        <circle
          key={name}
          className="terminal"
          cx={point.x}
          cy={point.y}
          r="5"
          onPointerDown={(event) => (element.kind === 'ground' ? onPointerDown(event) : onTerminal(event, point))}
        />
      ))}
      <Label element={element} at={labelAt} result={result} anchor="start" />
    </g>
  );
}

/** The ID and the voltage and current of a part, beside it. */
function Label({
  element,
  at,
  result,
  anchor,
}: {
  element: EditorElement;
  at: Point;
  result: CircuitResponse['elements'][number] | undefined;
  anchor: 'start' | 'middle';
}) {
  if (element.kind === 'ground') return null;
  return (
    <g transform={`translate(${at.x} ${at.y})`} pointerEvents="none">
      <text className="part-id" textAnchor={anchor}>
        {element.id}
      </text>
      {result && (
        <g transform={`translate(${anchor === 'middle' ? -60 : 0} 4)`}>
          <Value label="U" value={result.voltage} />
          <g transform="translate(0 20)">
            <Value label="I" value={result.current} />
          </g>
        </g>
      )}
    </g>
  );
}

/** A computed value on the canvas: set in KaTeX when known, greyed out when not. */
function Value({ label, value, small = false }: { label: string; value: CircuitValue; small?: boolean }) {
  const known = value.status === 'known' && value.result !== null;
  const text = value.status === 'relative' ? (value.text ?? 'relativ') : null;
  return (
    <foreignObject width={small ? 110 : 140} height={small ? 20 : 22} className="value-box">
      <div className={small ? 'canvas-value small' : 'canvas-value'} title={value.text ?? undefined}>
        {known ? (
          <>
            <span className="value-label">{label} =</span> <Katex latex={value.result!} />
          </>
        ) : text ? (
          <span className="value-relative">{text}</span>
        ) : (
          <span className="value-unknown">{label} = ?</span>
        )}
      </div>
    </foreignObject>
  );
}

function Inspector({
  element,
  result,
  onChange,
  onRemove,
}: {
  element: EditorElement;
  result: CircuitResponse['elements'][number] | undefined;
  onChange(change: Partial<EditorElement>): void;
  onRemove(): void;
}) {
  const parameters = element as unknown as Record<string, string | undefined>;
  return (
    <div className="inspector">
      <div className="inspector-head">
        <span className="inspector-id">{element.id}</span>
        <span className="inspector-kind">{KIND_LABELS[element.kind]}</span>
        <button className="inspector-delete" title="Löschen (Entf)" onClick={onRemove}>
          ✕
        </button>
      </div>

      {element.kind === 'wire' && (
        <label className="field">
          <span>Material</span>
          <select
            value={element.material ?? ''}
            onChange={(event) => onChange({ material: event.target.value || undefined })}
          >
            <option value="">— ideal —</option>
            {MATERIALS.map((material) => (
              <option key={material.id} value={material.id}>
                {material.label}
              </option>
            ))}
          </select>
        </label>
      )}

      {FIELDS[element.kind].map((field) => (
        <label className="field" key={field.key}>
          <span>{field.label}</span>
          <input
            type="text"
            spellCheck={false}
            autoComplete="off"
            value={parameters[field.key] ?? ''}
            placeholder={field.placeholder}
            onChange={(event) => onChange({ [field.key]: event.target.value })}
          />
        </label>
      ))}

      {result && (
        <div className="inspector-results">
          <ResultRow label="Spannung" value={result.voltage} />
          <ResultRow label="Strom" value={result.current} />
          <ResultRow label="Leistung" value={result.power} />
          {result.notes.map((note) => (
            <div className="inspector-note" key={note}>
              {note}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function ResultRow({ label, value }: { label: string; value: CircuitValue }) {
  return (
    <div className="result-line">
      <span className="result-line-label">{label}</span>
      {value.status === 'known' && value.result !== null ? (
        <Katex latex={value.result} />
      ) : (
        <span className={value.status === 'relative' ? 'value-relative' : 'value-unknown'}>
          {value.text ?? 'Unbekannt'}
        </span>
      )}
    </div>
  );
}
