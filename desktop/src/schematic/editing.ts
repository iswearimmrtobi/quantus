import { useRef, useState, type PointerEvent as ReactPointerEvent, type RefObject } from 'react';
import { drawnPath, inRectangle, movedBy, pointKey, rotatedQuarter, wireEnds, withPath, type Path, type Placeable, type Point } from './geometry';
import { attachments, followed, pulledEnd, type Attachment, type Merged } from './wiring';
import type { useCanvasView, View } from './view';

export type Drag<E> =
  /** `moved`: whether the pointer has left the grid point it started on; if not, it was a tap. */
  /** `attached`: the ends of other wires on the moving parts' pins, which follow them. */
  | { type: 'move'; from: Point; origins: Map<string, E>; attached: Attachment<E>[]; moved: boolean }
  /** `origin`: the wire as it was when the drag began. */
  | { type: 'end'; id: string; end: 'a' | 'b'; origin: E; moved: boolean }
  /**
   * `across`: whether the wire leaves sideways first, decided by the first step of the pointer.
   * `tapped`: the wire the drawing started on, chosen instead when the pointer is let go where it
   * was pressed. `path`: the wire about to be drawn.
   */
  | { type: 'draw'; from: Point; to: Point; across: boolean | null; path: Path; tapped?: E }
  | { type: 'marquee'; from: Point; to: Point }
  | { type: 'pan'; from: Point; origin: View };

interface Editing<E extends Placeable> {
  elements: E[];
  onChange(next: E[]): void;
  canvas: RefObject<SVGSVGElement | null>;
  view: ReturnType<typeof useCanvasView>;
  /** The pins of a part (both ends of a wire) at their places on the canvas. */
  pins(element: E): Point[];
  /** A new wire from `from` to `to` among `elements`. */
  newWire(elements: E[], from: Point, to: Point): E;
  /** A part's rotation after a quarter turn. */
  turned?(rotation: number): number;
  /** A part pressed and let go without moving it, such as a switch flipped. */
  onTap?(element: E): void;
  /** Whether a tap on the part only does `onTap`; its inspector then opens on a double click. */
  tapOnly?(element: E): boolean;
  /** The drawing tidied after a change: wires that meet end to end merged, say. */
  tidy?(elements: E[]): Merged<E>;
}

/** The longest pause between the two taps of a double click, ms. */
const DOUBLE_TAP = 400;

/**
 * Selecting, moving, turning and deleting parts, pulling a wire's end, drawing a wire from a pin
 * (an L, or between two pins a wire that turns halfway), the selection rectangle and moving the
 * canvas. The wires on a moved or turned part's pins follow it.
 */
export function useSchematicEditing<E extends Placeable>({ elements, onChange, canvas, view: canvasView, pins, newWire, turned, onTap, tapOnly, tidy }: Editing<E>) {
  const [selection, setSelection] = useState<string[]>([]);
  const [drag, setDrag] = useState<Drag<E> | null>(null);
  /** The part whose inspector is open; it stays open only while that part alone is selected. */
  const [inspectedId, setInspected] = useState<string | null>(null);
  const inspected = inspectedId !== null && selection.length === 1 && selection[0] === inspectedId ? inspectedId : null;
  /** The last tap on a part, to tell a double click. */
  const lastTap = useRef<{ id: string; time: number } | null>(null);
  const { view, setView, toDrawing, toGrid } = canvasView;

  /** Selects a part alone and opens its inspector. */
  const inspect = (id: string) => {
    setSelection([id]);
    setInspected(id);
  };
  const closeInspector = () => setInspected(null);

  const update = (id: string, change: Partial<E>) => onChange(elements.map((e) => (e.id === id ? { ...e, ...change } : e)));

  /**
   * `next` tidied (see `tidy`), with `select` or else the selection as the new selection: a wire
   * merged into another one is chosen as that one.
   */
  const tidied = (next: E[], select?: string[]): E[] => {
    const { elements: result, into } = tidy ? tidy(next) : { elements: next, into: new Map<string, string>() };
    if (select === undefined && !into.size) return result;
    const present = new Set(result.map((e) => e.id));
    setSelection([...new Set((select ?? selection).map((id) => into.get(id) ?? id))].filter((id) => present.has(id)));
    if (inspectedId !== null && into.has(inspectedId)) setInspected(null);
    return result;
  };

  /** `next`, the parts of `moving` moved or turned in it, with the wires on their pins following. */
  const following = (next: E[], moving: Iterable<string>): E[] => followed(next, attachments(elements, new Set(moving), pins), pins);

  const remove = (ids: string[]) => {
    onChange(tidied(elements.filter((e) => !ids.includes(e.id)), []));
    setInspected(null);
  };

  const rotateSelected = () => {
    if (!elements.some((e) => selection.includes(e.id))) return;
    onChange(tidied(following(rotatedQuarter(elements, selection, pins, turned), selection)));
  };

  const capture = (event: ReactPointerEvent) => canvas.current?.setPointerCapture(event.pointerId);

  // Only the left button works on parts; the others fall through to the canvas and move it.
  const startMove = (event: ReactPointerEvent, element: E) => {
    if (event.button !== 0) return;
    event.stopPropagation();
    capture(event);
    // Grabbing a part of the selection moves all of it; any other part becomes the selection.
    const group = selection.includes(element.id) ? selection : [element.id];
    setSelection(group);
    if (element.id !== inspectedId) setInspected(null);
    setDrag({
      type: 'move',
      from: toGrid(event),
      origins: new Map(elements.filter((e) => group.includes(e.id)).map((e) => [e.id, e])),
      attached: attachments(elements, new Set(group), pins),
      moved: false,
    });
  };

  const startEnd = (event: ReactPointerEvent, element: E, end: 'a' | 'b') => {
    if (event.button !== 0) return;
    event.stopPropagation();
    capture(event);
    setSelection([element.id]);
    if (element.id !== inspectedId) setInspected(null);
    setDrag({ type: 'end', id: element.id, end, origin: element, moved: false });
  };

  /** Draws a wire from `from`; pressed on a wire (`tapped`), a tap without moving chooses that wire. */
  const startDraw = (event: ReactPointerEvent, from: Point, tapped?: E) => {
    if (event.button !== 0) return;
    event.stopPropagation();
    capture(event);
    setDrag({ type: 'draw', from, to: from, across: null, path: [from, from], tapped });
  };

  /** Left on empty canvas draws a selection rectangle; the middle or right button moves the canvas. */
  const startCanvas = (event: ReactPointerEvent) => {
    capture(event);
    canvas.current?.focus();
    if (event.button === 1 || event.button === 2) {
      event.preventDefault();
      setDrag({ type: 'pan', from: { x: event.clientX, y: event.clientY }, origin: view });
      return;
    }
    const from = toDrawing(event);
    setSelection([]);
    setInspected(null);
    setDrag({ type: 'marquee', from, to: from });
  };

  const onMove = (event: ReactPointerEvent) => {
    if (!drag) return;
    if (drag.type === 'pan') {
      setView({ ...drag.origin, x: drag.origin.x + event.clientX - drag.from.x, y: drag.origin.y + event.clientY - drag.from.y });
      return;
    }
    if (drag.type === 'marquee') {
      setDrag({ ...drag, to: toDrawing(event) });
      return;
    }
    const at = toGrid(event);
    if (drag.type === 'draw') {
      if (at.x === drag.to.x && at.y === drag.to.y) return;
      const dx = Math.abs(at.x - drag.from.x);
      const dy = Math.abs(at.y - drag.from.y);
      const across = dx === 0 && dy === 0 ? null : (drag.across ?? dx >= dy);
      // Between two pins of parts the wire turns halfway; into the open, or to a wire, it is an L.
      const partPins = new Set(elements.filter((e) => e.kind !== 'wire').flatMap(pins).map(pointKey));
      const betweenPins = partPins.has(pointKey(drag.from)) && partPins.has(pointKey(at));
      setDrag({ ...drag, to: at, across, path: drawnPath(drag.from, at, across, betweenPins) });
    } else if (drag.type === 'end') {
      const wire = elements.find((e) => e.id === drag.id);
      if (!wire) return;
      const moved = pulledEnd(drag.origin, drag.end, at);
      const [before, after] = drag.end === 'a' ? [wireEnds(wire)[0], moved[0]] : [wireEnds(wire)[1], moved[moved.length - 1]];
      if (pointKey(before) === pointKey(after)) return;
      if (!drag.moved) setDrag({ ...drag, moved: true });
      onChange(elements.map((e) => (e.id === drag.id ? withPath(e, moved) : e)));
    } else {
      const [dx, dy] = [at.x - drag.from.x, at.y - drag.from.y];
      if (!drag.moved && dx === 0 && dy === 0) return;
      if (!drag.moved) setDrag({ ...drag, moved: true });
      onChange(followed(movedBy(elements, drag.origins, dx, dy), drag.attached, pins));
    }
  };

  const onUp = () => {
    if (drag?.type === 'draw' && pointKey(drag.from) !== pointKey(drag.to)) {
      const wire = withPath(newWire(elements, drag.from, drag.to), drag.path);
      onChange(tidied([...elements, wire], [wire.id]));
    } else if (drag && (drag.type === 'move' || drag.type === 'end') && drag.moved) {
      // Let go, wires that now meet end to end become one.
      onChange(tidied(elements));
    } else if (drag?.type === 'draw' && drag.tapped) {
      inspect(drag.tapped.id);
    } else if (drag?.type === 'marquee') {
      setSelection(inRectangle(elements, drag.from, drag.to, pins));
    } else if (drag?.type === 'end' && !drag.moved) {
      inspect(drag.id);
    } else if (drag?.type === 'move' && !drag.moved && drag.origins.size === 1) {
      tap([...drag.origins.values()][0]);
    }
    setDrag(null);
  };

  /**
   * A part pressed and let go without moving it: its inspector opens. A part that only taps (a
   * switch) does its tap instead, and opens its inspector on a double click; the second tap undoes
   * the first then, so the double click leaves the part as it was.
   */
  const tap = (element: E) => {
    const now = performance.now();
    const double = lastTap.current?.id === element.id && now - lastTap.current.time < DOUBLE_TAP;
    lastTap.current = double ? null : { id: element.id, time: now };
    if (!tapOnly?.(element)) {
      inspect(element.id);
      return;
    }
    onTap?.(element);
    if (double) inspect(element.id);
  };

  return {
    selection,
    setSelection,
    inspected,
    inspect,
    closeInspector,
    drag,
    setDrag,
    update,
    tidied,
    following,
    remove,
    rotateSelected,
    startMove,
    startEnd,
    startDraw,
    startCanvas,
    onMove,
    onUp,
  };
}
