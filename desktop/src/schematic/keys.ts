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

/** A key's name as shown: `key.ctrl+Z` is `Ctrl+Z`, the parts that are texts translated. */
export const keyName = (key: string, t: Translate) =>
  key
    .split('+')
    .map((part) => (part.startsWith('key.') ? t(part as TextKey) : part))
    .join('+');

/** Neither Ctrl, ⌘ nor Alt: those stay the app's own. */
const plain = (event: KeyboardEvent) => !event.ctrlKey && !event.metaKey && !event.altKey;

/** What the keys of every editor work on. */
export interface KeyActions {
  removeSelection(): void;
  rotateSelection(): void;
  /** Closes the inspector, else clears the selection; false when there was neither. */
  escape(): boolean;
  fit(): void;
  /** Return false when there was nothing to undo or redo. */
  undo(): boolean | void;
  redo(): boolean | void;
  /** Return false when nothing is chosen, or there is nothing to paste. */
  copy(): boolean;
  paste(): boolean | void;
  duplicate(): boolean | void;
  /** Moves the chosen parts by (dx, dy) grid steps; false when none is chosen. */
  nudge(dx: number, dy: number): boolean | void;
  view: ReturnType<typeof useCanvasView>;
}

/** The arrow keys' directions. */
const ARROWS: Record<string, [number, number]> = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, -1], ArrowDown: [0, 1] };

/** Ctrl, or ⌘ on a Mac, and neither Alt. */
const command = (event: KeyboardEvent) => (event.ctrlKey || event.metaKey) && !event.altKey;

/** The keys of every editor, after a mode's own. */
export function editorKeys(actions: KeyActions): KeyBinding[] {
  return [
    { keys: ['key.ctrl+Z'], hint: 'schematic.undoes', matches: (e) => command(e) && !e.shiftKey && e.key.toLowerCase() === 'z', run: actions.undo },
    {
      keys: ['key.ctrl+key.shift+Z', 'key.ctrl+Y'],
      hint: 'schematic.redoes',
      matches: (e) => command(e) && ((e.shiftKey && e.key.toLowerCase() === 'z') || e.key.toLowerCase() === 'y'),
      run: actions.redo,
    },
    { keys: ['key.ctrl+C'], hint: 'schematic.copies', matches: (e) => command(e) && !e.shiftKey && e.key.toLowerCase() === 'c', run: actions.copy },
    { keys: ['key.ctrl+V'], hint: 'schematic.pastes', matches: (e) => command(e) && !e.shiftKey && e.key.toLowerCase() === 'v', run: actions.paste },
    { keys: ['key.ctrl+D'], hint: 'schematic.duplicates', matches: (e) => command(e) && !e.shiftKey && e.key.toLowerCase() === 'd', run: actions.duplicate },
    { keys: ['key.delete'], hint: 'circuit.deletes', matches: (e) => e.key === 'Delete' || e.key === 'Backspace', run: actions.removeSelection },
    { keys: ['key.space', 'R'], hint: 'circuit.rotates', matches: (e) => plain(e) && (e.key === ' ' || e.key.toLowerCase() === 'r'), run: actions.rotateSelection },
    {
      keys: ['←', '→', '↑', '↓'],
      hint: 'schematic.moves',
      matches: (e) => plain(e) && e.key in ARROWS,
      run: (e) => {
        const [dx, dy] = ARROWS[e.key];
        const steps = e.shiftKey ? 5 : 1;
        return actions.nudge(dx * steps, dy * steps);
      },
    },
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
