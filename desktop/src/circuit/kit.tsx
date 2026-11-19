import type { CircuitResponse, CircuitValue } from '../api';
import { Katex } from '../components/Katex';
import { useT } from '../i18n';
import { opposite, sidesAround, wireBetween, type Rect, type Side } from '../schematic/geometry';
import { InspectorHead, SelectField, TextField } from '../schematic/inspector';
import type { PartKit } from '../schematic/kit';
import type { LabelRequest } from '../schematic/labels';
import {
  body,
  ends,
  freshId,
  HALF,
  idealSegments,
  isDiagonal,
  isIdealWire,
  KIND_LABELS,
  MATERIALS,
  newElement,
  obstacles,
  terminals,
  turnedBy,
  type EditorElement,
  type EditorKind,
  type Field,
  type Point,
} from './model';

/**
 * The parts of Electronics and Kirchhoff for the shared editor: sources, resistors, a
 * potentiometer, a load, wires, ground and the terminals A and B, their labels and inspector.
 */

export type ElementResult = CircuitResponse['elements'][number];

/** Where a part is grabbed, in its own coordinates: along its body, short of its terminals. */
const HIT: Rect = { x: -HALF + 8, y: -16, w: 2 * HALF - 16, h: 32 };

/** The kit of a mode offering `palette`. */
export function circuitKit(palette: EditorKind[]): PartKit<EditorElement, EditorKind> {
  return {
    palette,
    paletteLabel: (kind, t) => t(KIND_LABELS[kind]),
    paletteIcon: (kind) => (
      <svg viewBox="-48 -24 96 48" width="48" height="24">
        <Symbol kind={kind} />
      </svg>
    ),
    spacing: 3,
    // There are two terminals, A and B, and no third.
    canAdd: (elements, kind) => kind !== 'terminal' || elements.filter((e) => e.kind === 'terminal').length < 2,
    newPart: newElement,
    newWire: (elements, from, to) => wireBetween(freshId(elements, 'wire'), from, to),
    pins: ends,
    // A ground and a terminal are moved by their one point; no wire starts there.
    pinMoves: (element) => element.kind === 'ground' || element.kind === 'terminal',
    joining: idealSegments,
    wireJoins: isIdealWire,
    // The terminals A and B are marks, not lines, and make no junction.
    dotEnds: (elements) => elements.filter((e) => e.kind !== 'terminal').flatMap(ends),
    turned: (rotation) => turnedBy(rotation, 2),
    body,
    hit: () => HIT,
    obstacles: (element) => obstacles([element]),
    symbol: (element) => (
      <>
        <Symbol kind={element.kind} />
        {/* At 45° the terminals are √2 times as far out, so the leads run on to them. */}
        {isDiagonal(element.rotation) && <path d={`M${-HALF * Math.SQRT2} 0H${-HALF}M${HALF} 0H${HALF * Math.SQRT2}`} />}
      </>
    ),
    decoration: (element) =>
      element.kind === 'terminal' && (
        <text className="terminal-name" x={element.x + 12} y={element.y - 10}>
          {element.id}
        </text>
      ),
    keys: ({ elements, selection, onChange }) => [
      {
        // Shift+R turns each selected resistor by 45° about its middle; other parts stay as they are.
        keys: ['key.shiftR'],
        hint: 'circuit.tilts',
        matches: (event) => event.key.toLowerCase() === 'r' && event.shiftKey && !event.ctrlKey && !event.metaKey,
        run: () => {
          if (!elements.some((e) => selection.includes(e.id) && e.kind === 'resistor')) return;
          onChange(elements.map((e) => (selection.includes(e.id) && e.kind === 'resistor' ? { ...e, rotation: turnedBy(e.rotation, 1) } : e)));
        },
      },
    ],
  };
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
    case 'terminal':
      return <circle r="8" />;
    case 'wire':
      return <path d={`M${-HALF} 0H${HALF}`} />;
  }
}

export type Place = Pick<LabelRequest, 'anchor' | 'sides' | 'gap' | 'corner'>;

/**
 * Where a part's label would like to sit: under a horizontal part, right of a vertical one, on the
 * other side if a potentiometer's wiper is there; the layout takes the next free spot if that one
 * is not. An ideal wire has no drop, so it only gets a label once it has a material or a
 * resistance; the inspector shows the rest.
 */
export function partPlace(element: EditorElement): Place | null {
  if (element.kind === 'ground' || element.kind === 'terminal') return null;
  if (element.kind === 'wire') {
    if (!element.material && !element.rOverride?.trim()) return null;
    const { a, b } = terminals(element) as Record<'a' | 'b', Point>;
    const anchor = { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
    const across = Math.abs(b.x - a.x) >= Math.abs(b.y - a.y);
    return { anchor, gap: 8, sides: across ? sidesAround('above', 'right') : sidesAround('right', 'above') };
  }
  // A part at 45° counts with the direction it leans more towards: 45° as across, 135° as down.
  const vertical = Math.floor(element.rotation) % 2 === 1;
  const wiper = terminals(element).wiper;
  const flipped = wiper !== undefined && (vertical ? wiper.x > element.x : wiper.y > element.y);
  const [first, second]: Side[] = vertical ? ['right', 'below'] : ['below', 'right'];
  const sides = sidesAround(flipped ? opposite(first) : first, second);
  return { anchor: { x: element.x, y: element.y }, gap: 18, sides };
}

/** A node's voltage sits above and right of its point, clear of the wires that meet there. */
export const nodePlace = (point: Point): Place => ({
  anchor: point,
  gap: 8,
  corner: true,
  sides: ['above', 'below', 'right', 'left'],
});


/** A computed value on the canvas: set in KaTeX when known, amber when relative, greyed out when not. */
export function Value({ label, value }: { label: string; value: CircuitValue }) {
  const shown = value.status !== 'unknown' && value.result !== null;
  return (
    <div className="canvas-value" title={value.text ?? undefined}>
      <span className="value-label">{label} =</span>{' '}
      {shown ? (
        <Katex latex={value.result!} className={value.status === 'relative' ? 'value-relative' : undefined} />
      ) : (
        <span className="value-unknown">?</span>
      )}
    </div>
  );
}

export function CircuitInspector({
  element,
  fields,
  result,
  onChange,
  onRemove,
  onClose,
}: {
  element: EditorElement;
  fields: Field[];
  result: ElementResult | undefined;
  onChange(change: Partial<EditorElement>): void;
  onRemove(): void;
  onClose(): void;
}) {
  const { t, language } = useT();
  const parameters = element as unknown as Record<string, string | undefined>;
  return (
    <div className="inspector">
      <InspectorHead id={element.id} kind={t(KIND_LABELS[element.kind])} onRemove={onRemove} onClose={onClose} />

      {/* A mode with ideal wires only still offers the material when one is set, to take it off. */}
      {element.kind === 'wire' && (fields.length > 0 || element.material) && (
        <SelectField
          label={t('field.material')}
          value={element.material ?? ''}
          options={[{ value: '', label: t('circuit.ideal') }, ...MATERIALS.map((material) => ({ value: material.id, label: t(material.label) }))]}
          onChange={(material) => onChange({ material: material || undefined })}
        />
      )}

      {fields.map((field) => (
        <TextField
          key={field.key}
          label={t(field.label)}
          value={parameters[field.key] ?? ''}
          placeholder={field.placeholder.replace('{,}', language === 'de' ? '{,}' : '.')}
          onChange={(text) => onChange({ [field.key]: text })}
        />
      ))}

      {result && (
        <div className="inspector-results">
          <ResultRow label={t('result.voltage')} value={result.voltage} />
          <ResultRow label={t('result.current')} value={result.current} />
          <ResultRow label={t('result.power')} value={result.power} />
          {result.resistance && <ResultRow label={t('result.resistance')} value={result.resistance} />}
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
  const { t } = useT();
  return (
    <div className="result-line">
      <span className="result-line-label">{label}</span>
      {value.status === 'known' && value.result !== null ? (
        <Katex latex={value.result} />
      ) : (
        <span className={value.status === 'relative' ? 'value-relative' : 'value-unknown'}>
          {value.text ?? t('result.unknown')}
        </span>
      )}
    </div>
  );
}

