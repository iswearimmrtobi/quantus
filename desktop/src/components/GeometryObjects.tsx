import type { Geometry, GeometryEntry, GeometryObject } from '../api';
import { useT } from '../i18n';
import type { TextKey } from '../i18n/en';
import { freeName, newObject, type FormKind } from '../geometry';
import { colourStyle } from './GeometryPlot';
import { Katex } from './Katex';
import { MathField } from './MathField';

const KINDS: { kind: FormKind; label: TextKey }[] = [
  { kind: 'point', label: 'geometry.kind.point' },
  { kind: 'vector', label: 'geometry.kind.vector' },
];

interface Props {
  objects: GeometryObject[];
  /** The engine's answer to the scene, in the same order: each object as built, or its error. */
  built: Geometry['objects'] | null;
  onChange(objects: GeometryObject[]): void;
}

/** The colour index of each object in the view: built objects are drawn in scene order. */
export function colourIndices(built: Geometry['objects'] | null): (number | null)[] {
  let next = 0;
  return (built ?? []).map((object) => (object.error ? null : next++));
}

/**
 * Coordinate cells of a point or vector: two or three small formula fields. Tab moves through
 * them; an empty cell is 0, an empty z makes it a 2D input.
 */
function Cells({ entry, vector, onChange }: { entry: GeometryEntry; vector: boolean; onChange(entry: GeometryEntry): void }) {
  const cells = Array.isArray(entry) ? entry : ['', '', ''];
  const set = (index: number, value: string) => onChange(cells.map((cell, i) => (i === index ? value : cell)));
  return (
    <span className={vector ? 'geometry-cells vector' : 'geometry-cells'}>
      {cells.map((cell, index) => (
        <span key={index} className="geometry-cell-wrap">
          {!vector && index > 0 && <span className="geometry-bar">|</span>}
          <span className="matrix-cell geometry-cell">
            <MathField className="cell-input" value={cell} onChange={(value) => set(index, value)} onEnter={() => {}} autoFocus={false} />
          </span>
        </span>
      ))}
    </span>
  );
}

/** The form of one object, without its name. */
function Form({ object, onChange }: { object: GeometryObject; onChange(object: GeometryObject): void }) {
  switch (object.kind) {
    case 'point':
      return <Cells entry={object.at} vector={false} onChange={(at) => onChange({ ...object, at })} />;
    case 'vector':
      return <Cells entry={object.value} vector onChange={(value) => onChange({ ...object, value })} />;
    default:
      return null;
  }
}

/**
 * The scene as a list of forms, one per object, in the colour the view draws it in: its name,
 * its entries, and below them the object as the engine read it, or why it could not.
 */
export function GeometryObjects({ objects, built, onChange }: Props) {
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
              <Form object={object} onChange={(next) => replace(index, next)} />
              <button className="copy-btn geometry-remove" title={t('geometry.remove')} onClick={() => remove(index)}>
                ×
              </button>
            </div>
            {answer?.error ? (
              <span className="geometry-error">{answer.error}</span>
            ) : (
              answer?.latex && <Katex className="geometry-built" latex={answer.latex} />
            )}
          </div>
        );
      })}
    </section>
  );
}
