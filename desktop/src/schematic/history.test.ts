import { describe, expect, it } from 'vitest';
import { emptyHistory, GROUP_PAUSE, HISTORY_LIMIT, recorded, redone, same, undone } from './history';

describe('history', () => {
  it('undoes and redoes one change at a time', () => {
    let h = recorded(emptyHistory<number>(), 1, undefined, 0);
    h = recorded(h, 2, undefined, 10);
    const back = undone(h, 3)!;
    expect(back.value).toBe(2);
    const further = undone(back.history, 2)!;
    expect(further.value).toBe(1);
    expect(undone(further.history, 1)).toBeNull();
    const again = redone(further.history, 1)!;
    expect(again.value).toBe(2);
    expect(redone(again.history, 2)!.value).toBe(3);
  });

  it('keeps a group in a row as one entry, until a pause or another group', () => {
    let h = recorded(emptyHistory<string>(), 'a', 'field', 0);
    h = recorded(h, 'ab', 'field', 100);
    h = recorded(h, 'abc', 'field', 200);
    expect(h.past).toEqual(['a']);
    h = recorded(h, 'abcd', 'field', 200 + GROUP_PAUSE);
    expect(h.past).toEqual(['a', 'abcd']);
    h = recorded(h, 'x', 'other', 200 + GROUP_PAUSE);
    expect(h.past).toEqual(['a', 'abcd', 'x']);
  });

  it('keeps a sealed group however slow it is', () => {
    let h = recorded(emptyHistory<number>(), 0, 'pointer', 0);
    h = recorded(h, 1, 'pointer', 5 * GROUP_PAUSE, true);
    expect(h.past).toEqual([0]);
  });

  it('forgets what was undone once something new happens', () => {
    const h = recorded(emptyHistory<number>(), 1, undefined, 0);
    const back = undone(h, 2)!;
    expect(recorded(back.history, 1, undefined, 1).future).toEqual([]);
  });

  it('keeps at most the limit', () => {
    let h = emptyHistory<number>();
    for (let n = 0; n < HISTORY_LIMIT + 5; n += 1) h = recorded(h, n, undefined, n);
    expect(h.past).toHaveLength(HISTORY_LIMIT);
    expect(h.past[0]).toBe(5);
  });
});

describe('same', () => {
  it('takes lists of the same parts as the same drawing', () => {
    const part = { id: 'R1' };
    expect(same([], [])).toBe(true);
    expect(same([part], [part])).toBe(true);
    expect(same([part], [{ id: 'R1' }])).toBe(false);
    expect(same([part], [])).toBe(false);
  });
});
