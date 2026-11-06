import type { Geometry, GeometryEntry, GeometryObject } from '../api';
import { useT } from '../i18n';
import type { TextKey } from '../i18n';
import { formKindOf, freeName, plainBars, newObject, switchVariant, VARIANTS, type FormKind, type Variant } from '../geometry';
import { colourStyle } from './GeometryPlot';
import { Katex } from './Katex';
import { Steps } from './AnalysisView';
import { MathField } from './MathField';
import { Segmented } from './SettingsDialog';

const KINDS: { kind: FormKind; label: TextKey }[] = [
  { kind: 'point', label: 'geometry.kind.point' },
  { kind: 'vector', label: 'geometry.kind.vector' },
  { kind: 'line', label: 'geometry.kind.line' },
  { kind: 'plane', label: 'geometry.kind.plane' },
  { kind: 'sphere', label: 'geometry.kind.sphere' },
  { kind: 'circle', label: 'geometry.kind.circle' },
];

/** The name of each way an object can be given, for the switch above its form. */
const VARIANT_LABELS: Partial<Record<Variant, TextKey>> = {
  line: 'geometry.variant.line',
  lineThrough: 'geometry.variant.lineThrough',
  plane: 'geometry.variant.plane',
  planeNormal: 'geometry.variant.planeNormal',
  planeCoordinate: 'geometry.variant.planeCoordinate',
  planeThrough: 'geometry.variant.planeThrough',
  sphere: 'geometry.variant.sphere',
  sphereThrough: 'geometry.variant.sphereThrough',
  sphereFour: 'geometry.variant.sphereFour',
  circle: 'geometry.variant.circle',
  circleCut: 'geometry.variant.circleCut',
};

/** A pick of an object of the scene by name, from those of `kind` that were built. */
function ObjectPick({ value, kind, built, onChange }: { value: string; kind: string; built: Geometry['objects'] | null; onChange(name: string): void }) {
  const { t } = useT();
  const names = (built ?? []).filter((object) => object.kind === kind && !object.error).map((object) => object.name);
  return (
    <select className="setting-select geometry-pick" value={value} onChange={(event) => onChange(event.target.value)}>
      <option value="">{t('geometry.pick')}</option>
      {/* A name that is gone stays selectable, so the form shows what it holds. */}
      {value && !names.includes(value) && <option value={value}>{value}</option>}
      {names.map((name) => (
        <option key={name} value={name}>
          {name}
        </option>
      ))}
    </select>
  );
}

interface Props {
  objects: GeometryObject[];
  /** The engine's answer to the scene, in the same order: each object as built, or its error. */
  built: Geometry['objects'] | null;
  onChange(objects: GeometryObject[]): void;
  /** At the detail level `none` the steps of a construction are hidden. */
  hideSteps: boolean;
}

/** The colour index of each object in the view: built objects are drawn in scene order. */
export function colourIndices(built: Geometry['objects'] | null): (number | null)[] {
  let next = 0;
  return (built ?? []).map((object) => (object.error ? null : next++));
}

/** One small formula field for a number. */
function Cell({ value, onChange, wide = false, line = false }: { value: string; onChange(value: string): void; wide?: boolean; line?: boolean }) {
  return (
    <span className={`matrix-cell geometry-cell${wide ? ' wide' : ''}${line ? ' line' : ''}`}>
      <MathField className="cell-input" value={value} onChange={onChange} onEnter={() => {}} autoFocus={false} />
    </span>
  );
}

/**
 * Coordinate cells of a point or vector: two or three small formula fields. Tab moves through
 * them; an empty cell is 0, an empty z makes it a 2D input.
 */
function Cells({ cells, vector, onChange }: { cells: string[]; vector: boolean; onChange(cells: string[]): void }) {
  const set = (index: number, value: string) => onChange(cells.map((cell, i) => (i === index ? value : cell)));
  return (
    <span className={vector ? 'geometry-cells vector' : 'geometry-cells'}>
      {cells.map((cell, index) => (
        <span key={index} className="geometry-cell-wrap">
          {!vector && index > 0 && <span className="geometry-bar">|</span>}
          <Cell value={cell} onChange={(value) => set(index, value)} />
        </span>
      ))}
    </span>
  );
}

/**
 * A point or vector of a form: coordinate cells, or one field for a name or an expression of the
 * scene (`A`, `\vec{a}`, `B - A`). The small button switches between the two.
 */
function EntryField({ entry, vector, onChange }: { entry: GeometryEntry; vector: boolean; onChange(entry: GeometryEntry): void }) {
  const { t } = useT();
  const typed = typeof entry === 'string';
  return (
    <span className="geometry-entry">
      {typed ? <Cell value={entry} onChange={onChange} wide /> : <Cells cells={entry} vector={vector} onChange={onChange} />}
      <button
        className="geometry-entry-switch"
        title={t(typed ? 'geometry.entry.cells' : 'geometry.entry.name')}
        onClick={() => onChange(typed ? ['', '', ''] : '')}
      >
        {typed ? '( | )' : 'A'}
      </button>
    </span>
  );
}

/** A piece of the formula around the fields. */
const Text = ({ latex }: { latex: string }) => <Katex className="geometry-form-text" latex={latex} />;

/** The form of one object, without its name, laid out as its equation is written. */
function Form({ object, built, onChange }: { object: GeometryObject; built: Geometry['objects'] | null; onChange(object: GeometryObject): void }) {
  const { t } = useT();
  const point = (entry: GeometryEntry, set: (entry: GeometryEntry) => void) => <EntryField entry={entry} vector={false} onChange={set} />;
  const vector = (entry: GeometryEntry, set: (entry: GeometryEntry) => void) => <EntryField entry={entry} vector onChange={set} />;
  switch (object.kind) {
    case 'point':
      return point(object.at, (at) => onChange({ ...object, at }));
    case 'vector':
      return vector(object.value, (value) => onChange({ ...object, value }));
    case 'line':
      return (
        <>
          <Text latex="\vec{x} =" />
          {vector(object.support, (support) => onChange({ ...object, support }))}
          <Text latex="+ \lambda \cdot" />
          {vector(object.direction, (direction) => onChange({ ...object, direction }))}
        </>
      );
    case 'lineThrough':
      return (
        <>
          <Text latex="P =" />
          {point(object.a, (a) => onChange({ ...object, a }))}
          <Text latex="Q =" />
          {point(object.b, (b) => onChange({ ...object, b }))}
        </>
      );
    case 'plane':
      return (
        <>
          <Text latex="\vec{x} =" />
          {vector(object.support, (support) => onChange({ ...object, support }))}
          <Text latex="+ \lambda \cdot" />
          {vector(object.u, (u) => onChange({ ...object, u }))}
          <Text latex="+ \mu \cdot" />
          {vector(object.v, (v) => onChange({ ...object, v }))}
        </>
      );
    case 'planeNormal':
      return (
        <>
          <Text latex="\left(\vec{x} -\right." />
          {vector(object.support, (support) => onChange({ ...object, support }))}
          <Text latex="\left.\right) \cdot" />
          {vector(object.normal, (normal) => onChange({ ...object, normal }))}
          <Text latex="= 0" />
        </>
      );
    case 'planeCoordinate':
      return (
        <>
          <Cell value={object.a} onChange={(a) => onChange({ ...object, a })} />
          <Text latex="x +" />
          <Cell value={object.b} onChange={(b) => onChange({ ...object, b })} />
          <Text latex="y +" />
          <Cell value={object.c} onChange={(c) => onChange({ ...object, c })} />
          <Text latex="z =" />
          <Cell value={object.d} onChange={(d) => onChange({ ...object, d })} />
        </>
      );
    case 'planeThrough':
      return (
        <>
          <Text latex="P =" />
          {point(object.a, (a) => onChange({ ...object, a }))}
          <Text latex="Q =" />
          {point(object.b, (b) => onChange({ ...object, b }))}
          <Text latex="R =" />
          {point(object.c, (c) => onChange({ ...object, c }))}
        </>
      );
    case 'sphere':
      return (
        <>
          <Text latex="M =" />
          {point(object.centre, (centre) => onChange({ ...object, centre }))}
          <Text latex="r =" />
          <Cell value={object.radius} onChange={(radius) => onChange({ ...object, radius })} />
        </>
      );
    case 'sphereThrough':
      return (
        <>
          <Text latex="M =" />
          {point(object.centre, (centre) => onChange({ ...object, centre }))}
          <Text latex="P =" />
          {point(object.point, (p) => onChange({ ...object, point: p }))}
        </>
      );
    case 'sphereFour':
      return (
        <>
          <Text latex="P =" />
          {point(object.a, (a) => onChange({ ...object, a }))}
          <Text latex="Q =" />
          {point(object.b, (b) => onChange({ ...object, b }))}
          <Text latex="R =" />
          {point(object.c, (c) => onChange({ ...object, c }))}
          <Text latex="S =" />
          {point(object.d, (d) => onChange({ ...object, d }))}
        </>
      );
    case 'circle':
      return (
        <>
          <Text latex="M =" />
          {point(object.centre, (centre) => onChange({ ...object, centre }))}
          <Text latex="r =" />
          <Cell value={object.radius} onChange={(radius) => onChange({ ...object, radius })} />
          <Text latex="\vec{n} =" />
          <span title={t('geometry.circle.normal.hint')}>{vector(object.normal ?? ['', '', ''], (normal) => onChange({ ...object, normal }))}</span>
        </>
      );
    case 'circleCut':
      return (
        <>
          <ObjectPick value={object.sphere} kind="sphere" built={built} onChange={(sphere) => onChange({ ...object, sphere })} />
          <Text latex="\cap" />
          <ObjectPick value={object.plane} kind="plane" built={built} onChange={(plane) => onChange({ ...object, plane })} />
        </>
      );
    case 'typed':
      return <Cell value={object.latex} onChange={(latex) => onChange({ ...object, latex: plainBars(latex) })} line />;
    default:
      return null;
  }
}

/** The switch between the ways an object of its kind can be given, if there are several. */
function VariantSwitch({ object, onChange }: { object: GeometryObject; onChange(object: GeometryObject): void }) {
  const { t } = useT();
  const kind = formKindOf(object.kind);
  if (!kind || VARIANTS[kind].length < 2) return null;
  const choices = VARIANTS[kind].map((value) => ({ value, label: t(VARIANT_LABELS[value]!), hint: t(`${VARIANT_LABELS[value]!}.hint` as TextKey) }));
  return <Segmented choices={choices} value={object.kind as Variant} onChange={(variant) => onChange(switchVariant(object, variant))} />;
}

/**
 * The scene as a list of forms, one per object, in the colour the view draws it in: its name,
 * its entries, and below them the object as the engine read it, or why it could not.
 */
export function GeometryObjects({ objects, built, onChange, hideSteps }: Props) {
  const { t } = useT();
  const colours = colourIndices(built);
  const replace = (index: number, object: GeometryObject) => onChange(objects.map((o, i) => (i === index ? object : o)));
  const remove = (index: number) => onChange(objects.filter((_, i) => i !== index));
  const add = (kind: FormKind) => onChange([...objects, newObject(kind, freeName(kind, objects))]);

  return (
    <section className="section-card geometry-objects">
      <div className="geometry-add">
        <span className="geometry-add-label">{t('geometry.add')}</span>
        {KINDS.map(({ kind, label }) => (
          <button key={kind} className="palette-item" onClick={() => add(kind)}>
            {t(label)}
          </button>
        ))}
      </div>
      {objects.map((object, index) => {
        const answer = built?.[index];
        const colour = colours[index];
        return (
          <div key={index} className="geometry-object-row" style={colour === null || colour === undefined ? undefined : colourStyle(colour)}>
            <VariantSwitch object={object} onChange={(next) => replace(index, next)} />
            <div className="geometry-object-form">
              <span className="geometry-swatch" />
              {object.kind !== 'typed' && (
                <input
                  className="task-input geometry-name"
                  value={object.name ?? ''}
                  aria-label={t('geometry.name')}
                  title={t('geometry.name')}
                  onChange={(event) => replace(index, { ...object, name: event.target.value })}
                />
              )}
              <Form object={object} built={built} onChange={(next) => replace(index, next)} />
              <button className="copy-btn geometry-remove" title={t('geometry.remove')} onClick={() => remove(index)}>
                ×
              </button>
            </div>
            {answer?.error ? (
              <span className="geometry-error">{answer.error}</span>
            ) : (
              answer?.latex && <Katex className="geometry-built" latex={answer.latex} />
            )}
            {/* How a line through two points, a plane through three or a cut circle came about. */}
            {!hideSteps && answer && !answer.error && answer.steps.length > 0 && (
              <details className="geometry-object-steps">
                <summary>{t('steps')}</summary>
                <Steps steps={answer.steps} hidden={false} display />
              </details>
            )}
          </div>
        );
      })}
    </section>
  );
}
