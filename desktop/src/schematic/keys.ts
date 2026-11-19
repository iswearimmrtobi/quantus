import type { KeyboardEvent } from 'react';
import type { TextKey, Translate } from '../i18n';
import { ZOOM_STEP, type useCanvasView } from './view';

/**
 * A key of a focused canvas. One table drives both what the keys do and the list of them beside
 * the palette, so the two cannot drift apart.
 */
export interface KeyBinding {
  /** The keys as shown, one chip each: a text (`key.space`) or the key itself (`R`). */
  keys: string[];
  /** What it does, completing "‹keys› …". */
  hint: TextKey;
  matches(event: KeyboardEvent): boolean;
  /** Does it; returns false when there was nothing to do, so the key goes on (Esc to the full screen). */
  run(event: KeyboardEvent): boolean | void;
}

/** A key's name as shown. */
export const keyName = (key: string, t: Translate) => (key.startsWith('key.') ? t(key as TextKey) : key);

/** Neither Ctrl, ⌘ nor Alt: those stay the app's own. */
const plain = (event: KeyboardEvent) => !event.ctrlKey && !event.metaKey && !event.altKey;

/** What the keys of every editor work on. */
export interface KeyActions {
  removeSelection(): void;
  rotateSelection(): void;
  /** Closes the inspector, else clears the selection; false when there was neither. */
  escape(): boolean;
  fit(): void;
  view: ReturnType<typeof useCanvasView>;
}

/** The keys of every editor, after a mode's own. */
export function editorKeys(actions: KeyActions): KeyBinding[] {
  return [
    { keys: ['key.delete'], hint: 'circuit.deletes', matches: (e) => e.key === 'Delete' || e.key === 'Backspace', run: actions.removeSelection },
    { keys: ['key.space', 'R'], hint: 'circuit.rotates', matches: (e) => plain(e) && (e.key === ' ' || e.key.toLowerCase() === 'r'), run: actions.rotateSelection },
    { keys: ['Esc'], hint: 'schematic.escape', matches: (e) => e.key === 'Escape', run: actions.escape },
    { keys: ['key.home', '0'], hint: 'schematic.home', matches: (e) => plain(e) && (e.key === 'Home' || e.key === '0'), run: actions.fit },
    {
      keys: ['+', '−'],
      hint: 'schematic.zooms',
      matches: (e) => plain(e) && (e.key === '+' || e.key === '=' || e.key === '-'),
      run: (e) => actions.view.zoomBy(e.key === '-' ? 1 / ZOOM_STEP : ZOOM_STEP),
    },
  ];
}

/** Runs the first binding that matches; returns whether one took the key. */
export function runKey(bindings: KeyBinding[], event: KeyboardEvent): boolean {
  for (const binding of bindings) {
    if (!binding.matches(event)) continue;
    if (binding.run(event) === false) return false;
    event.preventDefault();
    return true;
  }
  return false;
}
