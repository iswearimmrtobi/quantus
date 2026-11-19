import type { GateType, LogicSymbols } from '../api';
import { Katex } from '../components/Katex';
import { useT, type Translate } from '../i18n';
import { nameLatex, readName } from '../logic';
import { GRID, pointKey, segmentBox, sidesAround, turnedBox, wireBetween, wireEnds, type Point, type Rect, type Side } from '../schematic/geometry';
import { CheckedTextField, InspectorHead, SelectField } from '../schematic/inspector';
import type { PartKit } from '../schematic/kit';
import type { CanvasLabel, LabelRequest } from '../schematic/labels';
import { guessedPicture, type Picture, type PictureColors } from '../schematic/picture';
import {
  allPins,
  bodyHalf,
  gateName,
  GATES,
  inputCount,
  inverted,
  LOGIC_PALETTE,
  localBody,
  MAX_GATE_INPUTS,
  newPart,
  numberNets,
  PART_LABELS,
  partPins,
  pastedPart,
  pinOffsets,
  single,
  toNetlist,
  withGate,
  type LogicPart,
  type PaletteKind,
} from './circuit';
import { simulate } from './simulate';

/**
 * The Logic mode's parts for the shared editor (design/digital-logic.md §10.2): switches, lamps,
 * constants and gates in IEC or ANSI symbols, their labels, inspector and picture.
 */

/** The kit of `symbols`, IEC rectangles or ANSI shapes; a part shows its simulated value. */
export function logicKit(symbols: LogicSymbols): PartKit<LogicPart, PaletteKind, boolean | null> {
  return {
    id: 'logic',
    palette: LOGIC_PALETTE,
    paletteLabel,
    paletteIcon: (kind) => (
      <svg viewBox="-46 -34 92 68" width="40" height="30">
        <PartSymbol part={paletteSample(kind)} symbols={symbols} />
      </svg>
    ),
    spacing: 5,
    newPart,
    newWire: (elements, from, to) => wireBetween(newPart(elements, 'wire', from).id, from, to),
    pasted: pastedPart,
    pins: allPins,
    joining: (parts) => parts.filter((p) => p.kind === 'wire').map(wireEnds),
    // The Logic mode's wires have no resistance: any of them can be joined in its middle.
    wireJoins: () => true,
    body: (part) => (part.kind === 'wire' ? segmentBox(...wireEnds(part), 2) : bodyBox(part)),
    hit: (part) => {
      // The body with a little room.
      const [x0, y0, x1, y1] = localBody(part);
      return { x: x0 - 4, y: y0 - 4, w: x1 - x0 + 8, h: y1 - y0 + 8 };
    },
    obstacles,
    symbol: (part, value) => <PartSymbol part={part} symbols={symbols} value={value} />,
    // A click flips a switch; a double click opens its inspector.
    tapOnly: (part) => part.kind === 'input',
    pictureStyle,
  };
}

/** The Logic mode's own CSS of a picture, after the shared one (`schematic/picture.tsx`). */
const pictureStyle = ({ ink, paper, muted, faint, on, lit }: PictureColors) => `
.symbol .logic-body, .symbol .logic-bubble, .symbol .logic-lamp, .symbol .logic-switch { fill: ${paper}; }
.symbol .logic-knob { fill: ${muted}; stroke: none; }
.logic-sign { fill: ${ink}; stroke: none; font-family: ui-monospace, Menlo, Consolas, monospace; font-size: 11px; text-anchor: middle; dominant-baseline: middle; }
.name { fill: ${ink}; font-family: 'Latin Modern Roman', 'Times New Roman', serif; font-size: 15px; text-anchor: middle; dominant-baseline: central; }
.name .letter { font-style: italic; }
.id { fill: ${faint}; font-family: ui-monospace, Menlo, Consolas, monospace; font-size: 10px; text-anchor: middle; dominant-baseline: central; }
${on ? `.wire.on, .symbol .logic-switch.on { stroke: ${on}; } .wire.off { stroke: ${muted}; } .symbol .logic-knob.on { fill: ${on}; }` : ''}
${lit ? `.symbol .logic-lamp.on { fill: ${lit}; }` : ''}
`;

/** The labels of the parts that have one: a switch's or lamp's name, a gate's id. */
export function logicLabels(parts: LogicPart[]): CanvasLabel[] {
  return parts.flatMap((part): CanvasLabel[] => {
    const place = labelPlace(part);
    if (!place) return [];
    const common = { ...place, id: part.id, leaderFrom: bodyBox(part) };
    if (part.kind === 'gate') {
      const svg = <text className="id">{part.id}</text>;
      return [{ ...common, className: 'logic-gate-label', content: part.id, svg, guess: { w: 7 * part.id.length + 6, h: 12 } }];
    }
    const name = part.name ?? '';
    const [base, index = ''] = name.split(/_(.*)/s);
    return [{ ...common, className: 'logic-name-label', content: <Katex latex={nameLatex(name)} />, svg: <NameText name={name} />, guess: { w: 9 * base.length + 6 * index.length + 6, h: 18 } }];
  });
}

/**
 * A circuit as a picture without an editor (the Gates card), in the app's colours or black on
 * white. With `switches` (by name) it shows the simulation as the editor does: each net at 1
 * bright, at 0 dim, the switches set and the lamps lit.
 */
export function staticPicture(parts: LogicPart[], symbols: LogicSymbols, theme: 'app' | 'export', switches?: Map<string, boolean>): Picture {
  const kit = logicKit(symbols);
  if (!switches) return guessedPicture(kit, parts, logicLabels(parts), theme);
  const nets = numberNets(parts);
  const netlist = toNetlist(parts, nets);
  const values = netlist ? simulate(netlist, switches) : new Map<number, boolean>();
  const at = (point: Point | undefined): boolean | null => {
    const net = point && nets.byPoint.get(pointKey(point));
    return net === undefined ? null : (values.get(net) ?? null);
  };
  return guessedPicture(kit, parts, logicLabels(parts), theme, {
    wireClass: (wire) => {
      const value = at(wireEnds(wire)[0]);
      return value === null ? undefined : value ? 'on' : 'off';
    },
    partState: (part) => {
      if (part.kind === 'input') return part.name ? (switches.get(part.name) ?? null) : null;
      if (part.kind === 'output') return at(partPins(part).inputs[0]);
      return null;
    },
  });
}

/** A switch's or lamp's name as SVG text: a letter in italics, a longer name upright, the index lowered. */
function NameText({ name }: { name: string }) {
  const [base, index] = name.split(/_(.*)/s);
  return (
    <text className="name">
      <tspan className={base.length === 1 ? 'letter' : undefined}>{base}</tspan>
      {index !== undefined && (
        <tspan dy="4" fontSize="10">
          {index}
        </tspan>
      )}
    </text>
  );
}

/** A part of a palette kind as the palette shows it. */
function paletteSample(kind: PaletteKind): LogicPart {
  return newPart([], kind, { x: 0, y: 0 });
}

function paletteLabel(kind: PaletteKind, t: Translate): string {
  return kind === 'input' || kind === 'output' || kind === 'constant' || kind === 'wire' ? t(PART_LABELS[kind]) : gateName(kind);
}

/** Where a part's label would like to sit: a switch's name on its left, a lamp's on its right, a gate's id above it. */
function labelPlace(part: LogicPart): Pick<LabelRequest, 'anchor' | 'sides' | 'gap'> | null {
  const box = bodyBox(part);
  const centre = { x: box.x + box.w / 2, y: box.y + box.h / 2 };
  const vertical = part.rotation % 2 === 1;
  if (part.kind === 'gate') {
    return { anchor: centre, sides: vertical ? sidesAround('right', 'above') : sidesAround('above', 'right'), gap: (vertical ? box.w : box.h) / 2 + 2 };
  }
  if (part.kind !== 'input' && part.kind !== 'output') return null;
  // The side away from the pin: a switch's pin points right when unturned, a lamp's left.
  const away: Side[] = ['left', 'above', 'right', 'below'];
  const turn = (part.rotation + (part.kind === 'output' ? 2 : 0)) % 4;
  const first = away[turn];
  // Across, the side of the pin comes last.
  const sides = first === 'left' || first === 'right' ? sidesAround(first, 'above', true) : sidesAround(first, 'left');
  return { anchor: centre, sides, gap: (first === 'left' || first === 'right' ? box.w : box.h) / 2 + 4 };
}

/** The box from (x0, y0) to (x1, y1) of a part's own coordinates, turned with it and moved to its place. */
const boxOfLocal = (part: LogicPart, x0: number, y0: number, x1: number, y1: number): Rect => turnedBox(part, [{ x: x0, y: y0 }, { x: x1, y: y1 }]);

/** A part's drawn body on the canvas. */
const bodyBox = (part: LogicPart): Rect => boxOfLocal(part, ...localBody(part));

/** What the labels keep clear of: a part's body, leads and pins; a wire's line. */
function obstacles(part: LogicPart): Rect[] {
  if (part.kind === 'wire') {
    const [a, b] = wireEnds(part);
    return [segmentBox(a, b, 2)];
  }
  const [left, , right] = localBody(part);
  // The leads run across from the pins to the body, in the part's own coordinates.
  const offsets = part.kind === 'gate' ? pinOffsets(inputCount(part)).map((o) => o * GRID) : [];
  const leads = [
    ...offsets.map((y) => boxOfLocal(part, -2 * GRID, y - 2, left, y + 2)),
    ...(part.kind === 'gate' ? [boxOfLocal(part, right, -2, 2 * GRID, 2)] : []),
    ...(part.kind === 'input' || part.kind === 'constant' ? [boxOfLocal(part, right, -2, GRID, 2)] : []),
    ...(part.kind === 'output' ? [boxOfLocal(part, -GRID, -2, left, 2)] : []),
  ];
  return [bodyBox(part), ...leads, ...allPins(part).map((pin) => segmentBox(pin, pin, 5))];
}

/** The sign in an IEC gate's box. */
const IEC_SIGN: Record<GateType, string> = { and: '&', nand: '&', or: '≥1', nor: '≥1', xor: '=1', xnor: '=1', not: '1', buffer: '1' };

/** The x of the back of an ANSI OR at height `y` of its half height `h`: a curve from (`back`, −h) over (`back` + 12, 0) to (`back`, h). */
const orBack = (back: number, h: number, y: number) => {
  const t = (y / h + 1) / 2;
  return back + 24 * t * (1 - t);
};

/**
 * A part in its own coordinates, unturned: a gate's inputs at (−2 grid, row), its output at (2 grid,
 * 0); a switch's and a constant's pin at (1 grid, 0), a lamp's at (−1 grid, 0).
 */
function PartSymbol({ part, symbols, value = null }: { part: LogicPart; symbols: LogicSymbols; value?: boolean | null }) {
  const state = value === null ? '' : value ? ' on' : ' off';
  switch (part.kind) {
    case 'wire':
      return <path d={`M${-2 * GRID} 0H${2 * GRID}`} />;
    case 'input':
      return (
        <>
          <path d={`M8 0H${GRID}`} />
          <rect className={`logic-switch${state}`} x="-16" y="-9" width="24" height="18" rx="9" />
          <circle className={`logic-knob${state}`} cx={value ? -1 : -7} cy="0" r="5" />
        </>
      );
    case 'constant':
      return (
        <>
          <path d={`M8 0H${GRID}`} />
          <rect x="-12" y="-9" width="20" height="18" rx="2" />
          <text className="logic-sign" x="-2" y="0">
            {part.value ? '1' : '0'}
          </text>
        </>
      );
    case 'output':
      return (
        <>
          <path d={`M${-GRID} 0H-9`} />
          <circle className={`logic-lamp${state}`} r="9" />
          <path d="M-6.4 -6.4L6.4 6.4M-6.4 6.4L6.4 -6.4" />
        </>
      );
    case 'gate':
      return <GateSymbol part={part} symbols={symbols} />;
  }
}

function GateSymbol({ part, symbols }: { part: LogicPart; symbols: LogicSymbols }) {
  const gate = part.gate ?? 'and';
  const h = bodyHalf(part);
  const offsets = pinOffsets(inputCount(part));
  const bubble = inverted(gate);
  const right = GRID;
  const out = <path d={`M${bubble ? right + 8 : right} 0H${2 * GRID}`} />;
  const mark = bubble && <circle className="logic-bubble" cx={right + 4} cy="0" r="4" />;
  if (symbols === 'iec') {
    const half = single(gate) ? 16 : h;
    return (
      <>
        {offsets.map((o) => (
          <path key={o} d={`M${-2 * GRID} ${o * GRID}H${-GRID}`} />
        ))}
        {out}
        <rect className="logic-body" x={-GRID} y={-half} width={2 * GRID} height={2 * half} />
        {mark}
        <text className="logic-sign" x="0" y={-half + 11} transform={`rotate(${-part.rotation * 90} 0 ${-half + 11})`}>
          {IEC_SIGN[gate]}
        </text>
      </>
    );
  }
  // ANSI: the distinctive shapes, from x = −grid to x = +grid.
  const leads = (to: (y: number) => number) =>
    offsets.map((o) => <path key={o} d={`M${-2 * GRID} ${o * GRID}H${to(o * GRID)}`} />);
  switch (gate) {
    case 'not':
    case 'buffer':
      return (
        <>
          {leads(() => -16)}
          {out}
          <path className="logic-body" d={`M-16 -14L${right} 0L-16 14Z`} />
          {mark}
        </>
      );
    case 'and':
    case 'nand':
      return (
        <>
          {leads(() => -GRID)}
          {out}
          <path className="logic-body" d={`M${-GRID} ${-h}H0A${GRID} ${h} 0 0 1 0 ${h}H${-GRID}Z`} />
          {mark}
        </>
      );
    case 'or':
    case 'nor':
      return (
        <>
          {leads((y) => orBack(-GRID, h, y))}
          {out}
          <path className="logic-body" d={`M${-GRID} ${-h}Q4 ${-h} ${GRID} 0Q4 ${h} ${-GRID} ${h}Q-8 0 ${-GRID} ${-h}Z`} />
          {mark}
        </>
      );
    case 'xor':
    case 'xnor':
      return (
        <>
          {leads((y) => orBack(-GRID, h, y))}
          {out}
          <path className="logic-body" d={`M-14 ${-h}Q6 ${-h} ${GRID} 0Q6 ${h} -14 ${h}Q-2 0 -14 ${-h}Z`} />
          <path d={`M${-GRID} ${-h}Q-8 0 ${-GRID} ${h}`} />
          {mark}
        </>
      );
  }
}

/** A part's settings: a switch's or lamp's name, a constant's value, a gate's type and number of inputs. */
export function LogicInspector({
  part,
  parts,
  onChange,
  onReplace,
  onRemove,
  onClose,
}: {
  part: LogicPart;
  parts: LogicPart[];
  onChange(change: Partial<LogicPart>): void;
  onReplace(part: LogicPart): void;
  onRemove(): void;
  onClose(): void;
}) {
  const { t } = useT();
  const kind = part.kind === 'gate' ? gateName(part.gate ?? 'and') : t(PART_LABELS[part.kind]);
  return (
    <div className="inspector">
      <InspectorHead id={part.id} kind={kind} onRemove={onRemove} onClose={onClose} />

      {(part.kind === 'input' || part.kind === 'output') && (
        <CheckedTextField
          key={part.id}
          label={t('logic.field.name')}
          value={part.name ?? ''}
          check={(text) => {
            // A name no other switch (or lamp) has.
            const name = readName(text);
            return name !== null && !parts.some((p) => p.kind === part.kind && p.id !== part.id && p.name === name) ? name : null;
          }}
          invalidTitle={t('logic.table.badName')}
          onChange={(name) => onChange({ name })}
        />
      )}

      {part.kind === 'constant' && (
        <SelectField
          label={t('logic.field.value')}
          value={part.value ? '1' : '0'}
          options={[
            { value: '0', label: '0' },
            { value: '1', label: '1' },
          ]}
          onChange={(value) => onChange({ value: value === '1' })}
        />
      )}

      {part.kind === 'gate' && (
        <>
          <SelectField
            label={t('logic.field.gate')}
            value={part.gate ?? 'and'}
            options={GATES.map((gate) => ({ value: gate, label: gateName(gate) }))}
            onChange={(gate) => onReplace(withGate(part, gate))}
          />
          {!single(part.gate) && (
            <SelectField
              label={t('logic.field.inputs')}
              value={String(inputCount(part))}
              options={Array.from({ length: MAX_GATE_INPUTS - 1 }, (_, k) => ({ value: String(k + 2), label: String(k + 2) }))}
              onChange={(n) => onChange({ inputs: Number(n) })}
            />
          )}
        </>
      )}
    </div>
  );
}

