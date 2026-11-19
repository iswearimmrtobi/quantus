import { describe, expect, it } from 'vitest';
import { copyParts, pasteInto, pasteParts, PASTE_STEP } from './clipboard';
import { freshIdOf, type Placeable } from './geometry';

const part = (id: string, x: number, y: number): Placeable => ({ id, kind: 'part', x, y, rotation: 0 });
const wire: Placeable = { id: 'W1', kind: 'wire', x: 0, y: 0, x2: 40, y2: 0, rotation: 0 };
const fresh = (p: Placeable, into: Placeable[]) => ({ ...p, id: freshIdOf(into.map((e) => e.id), p.kind === 'wire' ? 'W' : 'P') });

describe('clipboard', () => {
  it('pastes new parts, moved, with fresh ids', () => {
    const into = [part('P1', 0, 0), wire];
    const added = pasteInto([into[0], wire], into, 40, fresh);
    expect(added).toEqual([
      { id: 'P2', kind: 'part', x: 40, y: 40, rotation: 0 },
      { id: 'W2', kind: 'wire', x: 40, y: 40, x2: 80, y2: 40, rotation: 0 },
    ]);
  });

  it('leaves out what may not be pasted', () => {
    expect(pasteInto([part('P1', 0, 0)], [], 0, () => null)).toEqual([]);
  });

  it('pastes each time a step further, only into the same kind of editor', () => {
    const elements = [part('P1', 0, 0), part('P2', 100, 0)];
    expect(copyParts('a', elements, [])).toBe(false);
    expect(copyParts('a', elements, ['P2'])).toBe(true);
    expect(pasteParts('b', elements, fresh)).toBeNull();
    expect(pasteParts('a', elements, fresh)![0]).toMatchObject({ id: 'P3', x: 100 + PASTE_STEP });
    expect(pasteParts('a', elements, fresh)![0]).toMatchObject({ x: 100 + 2 * PASTE_STEP });
  });
});
