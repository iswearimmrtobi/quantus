import { useEffect, useState, type CSSProperties, type ReactNode, type RefObject } from 'react';
import { useT } from '../i18n';
import type { Rect } from './geometry';
import type { View } from './view';

/** The width of the inspector beside the chosen part, px. */
export const POPOVER_WIDTH = 260;

/**
 * A part's inspector, beside the part's `box` (drawing coordinates) over the canvas: to its right,
 * or to its left when there is no room, and inside the canvas.
 */
export function InspectorPopover({ box, view, canvas, children }: { box: Rect; view: View; canvas: RefObject<SVGSVGElement | null>; children: ReactNode }) {
  const stage = canvas.current?.getBoundingClientRect() ?? { width: 0, height: 0 };
  const [left, right] = [box.x * view.zoom + view.x, (box.x + box.w) * view.zoom + view.x];
  const top = Math.max(8, Math.min(box.y * view.zoom + view.y, stage.height - 120));
  const gap = 14;
  const x = right + gap + POPOVER_WIDTH <= stage.width - 8 ? right + gap : left - gap - POPOVER_WIDTH;
  const style: CSSProperties = {
    width: POPOVER_WIDTH,
    left: Math.max(8, Math.min(x, stage.width - POPOVER_WIDTH - 8)),
    top,
    maxHeight: `calc(100% - ${top + 8}px)`,
  };
  return (
    // Pointers on the inspector stay off the canvas behind it.
    <aside className="inspector-popover" style={style} onPointerDown={(event) => event.stopPropagation()}>
      {children}
    </aside>
  );
}

/** The head of an inspector: the part's id and kind, delete (a red bin) and close (✕). */
export function InspectorHead({ id, kind, onRemove, onClose }: { id: string; kind: string; onRemove(): void; onClose(): void }) {
  const { t } = useT();
  return (
    <div className="inspector-head">
      <span className="inspector-id">{id}</span>
      <span className="inspector-kind">{kind}</span>
      <button className="inspector-delete" title={`${t('circuit.delete')} (${t('key.delete')})`} aria-label={t('circuit.delete')} onClick={onRemove}>
        <svg viewBox="0 0 16 16" width="14" height="14" aria-hidden="true">
          <path d="M2.5 4h11M6 4V2.5h4V4M4 4l.7 9.5h6.6L12 4M6.6 6.5v4.5M9.4 6.5v4.5" />
        </svg>
      </button>
      <button className="inspector-close" title={t('circuit.close')} aria-label={t('circuit.close')} onClick={onClose}>
        ✕
      </button>
    </div>
  );
}

/** A labelled text input of an inspector. */
export function TextField({ label, value, placeholder, invalid, title, onChange }: { label: string; value: string; placeholder?: string; invalid?: boolean; title?: string; onChange(text: string): void }) {
  return (
    <label className="field">
      <span>{label}</span>
      <input
        type="text"
        spellCheck={false}
        autoComplete="off"
        className={invalid ? 'invalid' : undefined}
        title={title}
        value={value}
        placeholder={placeholder}
        onChange={(event) => onChange(event.target.value)}
      />
    </label>
  );
}

/** A labelled choice of an inspector. */
export function SelectField<T extends string>({ label, value, options, onChange }: { label: string; value: T; options: { value: T; label: string }[]; onChange(value: T): void }) {
  return (
    <label className="field">
      <span>{label}</span>
      <select value={value} onChange={(event) => onChange(event.target.value as T)}>
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
    </label>
  );
}

/**
 * A text input that only takes what `check` reads (a value other than null); the text stays as
 * typed meanwhile, marked invalid with `invalidTitle` as its hint. Give it a `key` per part, so
 * that it starts afresh for another one.
 */
export function CheckedTextField<T>({ label, value, check, invalidTitle, onChange }: { label: string; value: string; check(text: string): T | null; invalidTitle: string; onChange(value: T): void }) {
  const [text, setText] = useState(value);
  useEffect(() => setText(value), [value]);
  const valid = check(text) !== null;
  return (
    <TextField
      label={label}
      value={text}
      invalid={!valid}
      title={valid ? undefined : invalidTitle}
      onChange={(next) => {
        setText(next);
        const read = check(next);
        if (read !== null) onChange(read);
      }}
    />
  );
}
