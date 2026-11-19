import { useRef, useState, type KeyboardEvent, type PointerEvent as ReactPointerEvent, type RefObject } from 'react';
import { corner, inRectangle, movedBy, pointKey, rotatedQuarter, type Placeable, type Point } from './geometry';
import { zoomKey, type useCanvasView, type View } from './view';

export type Drag<E> =
  /** `moved`: whether the pointer has left the grid point it started on; if not, it was a tap. */
  | { type: 'move'; from: Point; origins: Map<string, E>; moved: boolean }
  | { type: 'end'; id: string; end: 'a' | 'b'; moved: boolean }
  /**
   * `across`: whether the wire leaves sideways first, decided by the first step of the pointer.
   * `tapped`: the wire the drawing started on, chosen instead when the pointer is let go where it
   * was pressed.
   */
  | { type: 'draw'; from: Point; to: Point; across: boolean | null; tapped?: E }
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
}

/** The longest pause between the two taps of a double click, ms. */
const DOUBLE_TAP = 400;

/**
 * Selecting, moving, turning and deleting parts, pulling a wire's end, drawing a wire from a pin
 * (an L of up to two wires), the selection rectangle and moving the canvas.
 */
export function useSchematicEditing<E extends Placeable>({ elements, onChange, canvas, view: canvasView, pins, newWire, turned, onTap, tapOnly }: Editing<E>) {
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

  const remove = (ids: string[]) => {
    onChange(elements.filter((e) => !ids.includes(e.id)));
    setSelection([]);
    setInspected(null);
  };

  const rotateSelected = () => {
    if (!elements.some((e) => selection.includes(e.id))) return;
    onChange(rotatedQuarter(elements, selection, pins, turned));
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
    setDrag({ type: 'move', from: toGrid(event), origins: new Map(elements.filter((e) => group.includes(e.id)).map((e) => [e.id, e])), moved: false });
  };

  const startEnd = (event: ReactPointerEvent, element: E, end: 'a' | 'b') => {
    if (event.button !== 0) return;
    event.stopPropagation();
    capture(event);
    setSelection([element.id]);
    if (element.id !== inspectedId) setInspected(null);
    setDrag({ type: 'end', id: element.id, end, moved: false });
  };

  /** Draws a wire from `from`; pressed on a wire (`tapped`), a tap without moving chooses that wire. */
  const startDraw = (event: ReactPointerEvent, from: Point, tapped?: E) => {
    if (event.button !== 0) return;
    event.stopPropagation();
    capture(event);
    setDrag({ type: 'draw', from, to: from, across: null, tapped });
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
      setDrag({ ...drag, to: at, across });
    } else if (drag.type === 'end') {
      // The end moves along the axis it is pulled most, so the wire stays across or down.
      const wire = elements.find((e) => e.id === drag.id);
      if (!wire) return;
      const other = drag.end === 'a' ? { x: wire.x2 ?? wire.x, y: wire.y2 ?? wire.y } : { x: wire.x, y: wire.y };
      const end = Math.abs(at.x - other.x) >= Math.abs(at.y - other.y) ? { x: at.x, y: other.y } : { x: other.x, y: at.y };
      const current = drag.end === 'a' ? { x: wire.x, y: wire.y } : { x: wire.x2 ?? wire.x, y: wire.y2 ?? wire.y };
      if (pointKey(end) === pointKey(current)) return;
      if (!drag.moved) setDrag({ ...drag, moved: true });
      update(drag.id, (drag.end === 'a' ? end : { x2: end.x, y2: end.y }) as Partial<E>);
    } else {
      const [dx, dy] = [at.x - drag.from.x, at.y - drag.from.y];
      if (!drag.moved && dx === 0 && dy === 0) return;
      if (!drag.moved) setDrag({ ...drag, moved: true });
      onChange(movedBy(elements, drag.origins, dx, dy));
    }
  };

  const onUp = () => {
    if (drag?.type === 'draw' && pointKey(drag.from) !== pointKey(drag.to)) {
      const bend = corner(drag.from, drag.to, drag.across);
      const next = [...elements];
      for (const [from, to] of [
        [drag.from, bend],
        [bend, drag.to],
      ]) {
        if (pointKey(from) === pointKey(to)) continue;
        next.push(newWire(next, from, to));
      }
      onChange(next);
      setSelection(next.slice(elements.length).map((e) => e.id));
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

  /**
   * The keys of a focused canvas: Del deletes the selection, Space or R turns it, Esc closes the
   * inspector and then clears the selection, Home calls `home`, and the zoom keys. `extra` takes a
   * mode's own keys first and returns whether it used the key.
   */
  const keyDown = (event: KeyboardEvent, { home, fit, extra }: { home(): void; fit(): void; extra?(event: KeyboardEvent): boolean }) => {
    if (extra?.(event)) {
      // Taken by the mode.
    } else if (event.key === 'Delete' || event.key === 'Backspace') {
      remove(selection);
    } else if (event.key === ' ' || event.key.toLowerCase() === 'r') {
      rotateSelected();
    } else if (event.key === 'Escape' && inspected) {
      closeInspector();
    } else if (event.key === 'Escape' && selection.length) {
      // With nothing selected, Esc is left to the full-screen view.
      setSelection([]);
    } else if (event.key === 'Home') {
      home();
    } else if (!zoomKey(event, canvasView, fit)) {
      return;
    }
    event.preventDefault();
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
    remove,
    rotateSelected,
    startMove,
    startEnd,
    startDraw,
    startCanvas,
    onMove,
    onUp,
    keyDown,
  };
}
