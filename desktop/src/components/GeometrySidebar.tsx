import { useEffect, useRef, useState } from 'react';
import type { ReactNode } from 'react';
import type { Geometry, GeometryObject } from '../api';
import { freeName, newObject, withTyped, type FormKind } from '../geometry';
import { useT } from '../i18n';
import { colourIndices, KINDS, ObjectEditor } from './GeometryObjects';
import { colourStyle } from './GeometryPlot';
import { Katex } from './Katex';
import { MathField } from './MathField';

/** An open eye, or a struck one for a hidden object. */
function Eye({ shut }: { shut: boolean }) {
  return (
    <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.4" aria-hidden>
      <path d="M1.5 8s2.4-4.5 6.5-4.5S14.5 8 14.5 8 12.1 12.5 8 12.5 1.5 8 1.5 8z" />
      <circle cx="8" cy="8" r="2" />
      {shut && <path d="M2.5 13.5l11-11" />}
    </svg>
  );
}

interface Props {
  objects: GeometryObject[];
  /** The engine's answer to the scene, in the same order: each object as built, or its error. */
  built: Geometry['objects'] | null;
  onChange(objects: GeometryObject[]): void;
  /** The row whose editor is open. */
  open: number | null;
  onOpen(index: number | null): void;
  /** The row the pointer is over, so the view can highlight its object. */
  onHover(index: number | null): void;
  /** At the detail level `none` the steps of a construction are hidden. */
  hideSteps: boolean;
  /** An example for the typed field. */
  placeholder?: string;
  /** Beside the typed field: the button that folds the sidebar. */
  headEnd?: ReactNode;
  /** Under the list: an error of the whole scene. */
  error?: ReactNode;
  /** At the foot: the task. */
  footer?: ReactNode;
}

/**
 * The scene in the sidebar: a typed line and a + menu on top, one compact row per object
 * (colour, the object as built, hide and remove), the open row's editor under it, and the task
 * at the foot.
 */
export function GeometrySidebar({ objects, built, onChange, open, onOpen, onHover, hideSteps, placeholder, headEnd, error, footer }: Props) {
  const { t } = useT();
  const colours = colourIndices(built);
  /** The typed line not yet entered. */
  const [typed, setTyped] = useState('');
  const enterTyped = () => {
    if (!typed.trim()) return;
    onChange(withTyped(objects, typed));
    setTyped('');
  };

  const [adding, setAdding] = useState(false);
  const menu = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!adding) return;
    const close = (event: PointerEvent | KeyboardEvent) => {
      if (event instanceof KeyboardEvent ? event.key === 'Escape' : !menu.current?.contains(event.target as Node)) setAdding(false);
    };
    document.addEventListener('pointerdown', close);
    document.addEventListener('keydown', close);
    return () => {
      document.removeEventListener('pointerdown', close);
      document.removeEventListener('keydown', close);
    };
  }, [adding]);
  const add = (kind: FormKind) => {
    onChange([...objects, newObject(kind, freeName(kind, objects))]);
    onOpen(objects.length);
    setAdding(false);
  };

  // The open row comes into sight, also when it was opened by a click in the view.
  const openRow = useRef<HTMLLIElement>(null);
  useEffect(() => {
    openRow.current?.scrollIntoView({ block: 'nearest' });
  }, [open]);

  const replace = (index: number, object: GeometryObject) => onChange(objects.map((o, i) => (i === index ? object : o)));
  const remove = (index: number) => {
    onChange(objects.filter((_, i) => i !== index));
    if (open !== null && index <= open) onOpen(index === open ? null : open - 1);
    onHover(null);
  };
  /** Moves an object one place; the order is the build order, so its editor goes along. */
  const move = (index: number, by: -1 | 1) => {
    const to = index + by;
    if (to < 0 || to >= objects.length) return;
    const next = [...objects];
    [next[index], next[to]] = [next[to], next[index]];
    onChange(next);
    onOpen(to);
  };
  const toggleHidden = (index: number) => {
    const { hidden, ...object } = objects[index];
    replace(index, hidden ? (object as GeometryObject) : { ...objects[index], hidden: true });
  };

  return (
    <aside className="geometry-sidebar">
      <div className="geometry-sidebar-head">
        <div className="input-row geometry-typed" title={t('geometry.typed.hint')}>
          <MathField value={typed} onChange={setTyped} onEnter={enterTyped} placeholder={placeholder} />
        </div>
        <div className="geometry-add" ref={menu}>
          <button className="copy-btn geometry-add-button" title={t('geometry.add.menu')} aria-label={t('geometry.add.menu')} aria-expanded={adding} onClick={() => setAdding((a) => !a)}>
            +
          </button>
          {adding && (
            <div className="geometry-add-menu" role="menu">
              {KINDS.map(({ kind, label }) => (
                <button key={kind} role="menuitem" onClick={() => add(kind)}>
                  {t(label)}
                </button>
              ))}
            </div>
          )}
        </div>
        {headEnd}
      </div>
      {error}
      {!objects.length && <p className="analysis-empty">{t('geometry.empty')}</p>}
      <ul className="geometry-list">
        {objects.map((object, index) => {
          const answer = built?.[index];
          const colour = colours[index];
          const isOpen = open === index;
          const name = answer?.name || object.name || '';
          const toggle = () => onOpen(isOpen ? null : index);
          return (
            <li
              key={index}
              ref={isOpen ? openRow : undefined}
              className={`geometry-row${isOpen ? ' open' : ''}${object.hidden ? ' hidden' : ''}`}
              style={colour === null || colour === undefined ? undefined : colourStyle(colour)}
              onMouseEnter={() => onHover(index)}
              onMouseLeave={() => onHover(null)}
              onKeyDown={(event) => {
                if (event.key === 'Escape' && isOpen) {
                  event.stopPropagation();
                  onOpen(null);
                }
              }}
            >
              <div
                className="geometry-row-head"
                role="button"
                tabIndex={0}
                aria-expanded={isOpen}
                onClick={toggle}
                onKeyDown={(event) => {
                  if (event.target === event.currentTarget && (event.key === 'Enter' || event.key === ' ')) {
                    event.preventDefault();
                    toggle();
                  }
                }}
              >
                <span className="geometry-swatch" />
                <span className="geometry-row-formula">
                  {answer?.latex && !answer.error ? (
                    <Katex latex={answer.latex} />
                  ) : (
                    <>
                      {name && <Katex className="geometry-row-name" latex={name} />}
                      {answer?.error && <span className="geometry-row-error">{answer.error}</span>}
                    </>
                  )}
                </span>
                <button
                  className="copy-btn geometry-row-button"
                  title={t(object.hidden ? 'geometry.sidebar.show' : 'geometry.sidebar.hide')}
                  aria-label={t(object.hidden ? 'geometry.sidebar.show' : 'geometry.sidebar.hide')}
                  aria-pressed={!!object.hidden}
                  onClick={(event) => {
                    event.stopPropagation();
                    toggleHidden(index);
                  }}
                >
                  <Eye shut={!!object.hidden} />
                </button>
                <button
                  className="copy-btn geometry-row-button"
                  title={t('geometry.remove')}
                  aria-label={t('geometry.remove')}
                  onClick={(event) => {
                    event.stopPropagation();
                    remove(index);
                  }}
                >
                  ×
                </button>
              </div>
              {isOpen && (
                <>
                  <ObjectEditor object={object} answer={answer} built={built} onChange={(next) => replace(index, next)} hideSteps={hideSteps} />
                  <div className="geometry-row-moves">
                    <button className="copy-btn" disabled={index === 0} onClick={() => move(index, -1)}>
                      ↑ {t('geometry.sidebar.moveUp')}
                    </button>
                    <button className="copy-btn" disabled={index === objects.length - 1} onClick={() => move(index, 1)}>
                      ↓ {t('geometry.sidebar.moveDown')}
                    </button>
                  </div>
                </>
              )}
            </li>
          );
        })}
      </ul>
      {footer}
    </aside>
  );
}
