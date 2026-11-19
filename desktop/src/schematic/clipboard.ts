import { GRID, type Placeable } from './geometry';

/**
 * Copy and paste within the editors. The clipboard is the app's own, not the system's: it holds
 * parts of one kind of editor (`kit`), Electronics and Kirchhoff sharing theirs.
 */

interface Clip {
  kit: string;
  parts: Placeable[];
  /** How often it was pasted, so that each paste lands a little further off. */
  pastes: number;
}

let clip: Clip | null = null;

/** Puts the parts with the ids of `selection` on the clipboard; false when none is chosen. */
export function copyParts<E extends Placeable>(kit: string, elements: E[], selection: string[]): boolean {
  const parts = elements.filter((e) => selection.includes(e.id));
  if (!parts.length) return false;
  clip = { kit, parts, pastes: 0 };
  return true;
}

/** How far each paste lands from the one before, in drawing units. */
export const PASTE_STEP = 2 * GRID;

/**
 * `parts` added to `into`, each made new by `pasted` (a fresh id, a free name; null leaves it
 * out) and moved by `offset` across and down. Returns the new parts.
 */
export function pasteInto<E extends Placeable>(parts: E[], into: E[], offset: number, pasted: (part: E, into: E[]) => E | null): E[] {
  const added: E[] = [];
  for (const part of parts) {
    const fresh = pasted(part, [...into, ...added]);
    if (!fresh) continue;
    const moved = { ...fresh, x: fresh.x + offset, y: fresh.y + offset };
    if (fresh.x2 !== undefined) moved.x2 = fresh.x2 + offset;
    if (fresh.y2 !== undefined) moved.y2 = fresh.y2 + offset;
    added.push(moved);
  }
  return added;
}

/** The clipboard's parts pasted into `elements`, if it holds parts of `kit`; null when not. */
export function pasteParts<E extends Placeable>(kit: string, elements: E[], pasted: (part: E, into: E[]) => E | null): E[] | null {
  if (!clip || clip.kit !== kit) return null;
  clip.pastes += 1;
  const added = pasteInto(clip.parts as E[], elements, clip.pastes * PASTE_STEP, pasted);
  return added.length ? added : null;
}
