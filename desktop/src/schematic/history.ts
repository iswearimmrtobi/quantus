import { useEffect, useRef, useState } from 'react';

/**
 * Undo and redo of an editor's drawing. Changes come in groups: the steps of one drag, the keys
 * typed into one field, a burst of arrow keys are one entry each, so one undo takes back all of
 * it. A change from outside the editor (a Kirchhoff lead flipped on the overlay) is an entry too.
 */

/** How many entries are kept. */
export const HISTORY_LIMIT = 100;
/** A group ends after this long without a change, ms; a drag's ends when the pointer is let go. */
export const GROUP_PAUSE = 1000;

export interface History<T> {
  past: T[];
  future: T[];
  /** The group of the last change and when it came, to add the next one of it to the same entry. */
  group: { name: string; at: number } | null;
}

export const emptyHistory = <T>(): History<T> => ({ past: [], future: [], group: null });

/** Whether two drawings are the same: the same value, or lists of the same parts. */
export function same<T>(a: T, b: T): boolean {
  if (a === b) return true;
  return Array.isArray(a) && Array.isArray(b) && a.length === b.length && a.every((x, i) => x === b[i]);
}

/**
 * The history after the drawing went from `before` to something new, in `group` (if any) at time
 * `now`: a new entry, unless the change continues the group of the last one.
 */
export function recorded<T>(history: History<T>, before: T, group: string | undefined, now: number, sealed = false): History<T> {
  const continues = group !== undefined && history.group?.name === group && (sealed || now - history.group.at < GROUP_PAUSE);
  if (continues) return { ...history, future: [], group: { name: group, at: now } };
  return { past: [...history.past, before].slice(-HISTORY_LIMIT), future: [], group: group === undefined ? null : { name: group, at: now } };
}

/** One step back from `current`: the drawing to show and the history then; null with nothing to undo. */
export function undone<T>(history: History<T>, current: T): { value: T; history: History<T> } | null {
  if (!history.past.length) return null;
  const value = history.past[history.past.length - 1];
  return { value, history: { past: history.past.slice(0, -1), future: [current, ...history.future], group: null } };
}

/** One step forward again from `current`; null with nothing to redo. */
export function redone<T>(history: History<T>, current: T): { value: T; history: History<T> } | null {
  if (!history.future.length) return null;
  const [value, ...future] = history.future;
  return { value, history: { past: [...history.past, current].slice(-HISTORY_LIMIT), future, group: null } };
}

/**
 * The history of `value`, which `onChange` changes. `change` is the editor's way to change it; a
 * new `resetKey` (a file loaded, a circuit drawn anew) starts the history afresh.
 */
export function useHistory<T>(value: T, onChange: (next: T) => void, resetKey: unknown) {
  const history = useRef<History<T>>(emptyHistory());
  /** What the editor last set, to tell its own changes from those from outside. */
  const expected = useRef(value);
  const previous = useRef(value);
  /** Whether a drag is on: its group lasts until `seal`, however slow the pointer. */
  const holding = useRef(false);
  const [, setVersion] = useState(0);
  const bump = () => setVersion((n) => n + 1);

  useEffect(() => {
    history.current = emptyHistory();
    // What comes with the reset (the loaded file) is no change to undo.
    expected.current = value;
    previous.current = value;
    bump();
  }, [resetKey]);

  useEffect(() => {
    if (!same(value, expected.current) && !same(value, previous.current)) {
      history.current = recorded(history.current, previous.current, undefined, performance.now());
      bump();
    }
    previous.current = value;
    expected.current = value;
  }, [value]);

  const set = (next: T) => {
    expected.current = next;
    onChange(next);
  };

  return {
    /** Changes the drawing; changes of one `group` in a row make one entry. */
    change(next: T, group?: string) {
      if (same(next, value)) return;
      const sealed = group === 'pointer' && holding.current;
      if (group === 'pointer') holding.current = true;
      history.current = recorded(history.current, value, group, performance.now(), sealed);
      set(next);
      bump();
    },
    /** Ends the group of a drag. */
    seal() {
      holding.current = false;
      if (history.current.group?.name === 'pointer') history.current = { ...history.current, group: null };
    },
    undo() {
      const step = undone(history.current, value);
      if (!step) return false;
      history.current = step.history;
      set(step.value);
      bump();
    },
    redo() {
      const step = redone(history.current, value);
      if (!step) return false;
      history.current = step.history;
      set(step.value);
      bump();
    },
    canUndo: history.current.past.length > 0,
    canRedo: history.current.future.length > 0,
  };
}

export type HistoryHandle = ReturnType<typeof useHistory>;
