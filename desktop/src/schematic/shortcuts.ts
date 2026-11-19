import type { Shortcut } from '../shell';

/**
 * The menu's shortcuts for a focused editor canvas. In the app, Ctrl+D is the menu's *Next result
 * form*, whose key the canvas never sees; while a canvas has the focus, the shortcut goes to it
 * first (Ctrl+D duplicates there), and to the calculator only when the canvas leaves it.
 */

let handler: ((shortcut: Shortcut) => boolean) | null = null;

/** Sets what the focused canvas does with the menu's shortcuts; null when it loses the focus. */
export function setCanvasShortcuts(next: ((shortcut: Shortcut) => boolean) | null) {
  handler = next;
}

/** Hands a shortcut to the focused canvas; true when it took it. */
export const canvasShortcut = (shortcut: Shortcut): boolean => handler?.(shortcut) ?? false;
