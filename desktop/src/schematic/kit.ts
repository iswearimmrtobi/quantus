import type { ReactNode } from 'react';
import type { Translate } from '../i18n';
import type { Placeable, Point, Rect, Segment } from './geometry';
import type { KeyBinding } from './keys';
import type { PictureColors } from './picture';

/** What a mode's own keys work on. */
export interface KitContext<E> {
  elements: E[];
  selection: string[];
  onChange(next: E[]): void;
}

/**
 * What the shared editor (`SchematicEditor`) needs to know about a mode's parts: which there are,
 * where their pins and bodies lie, how they are drawn. Everything else (the canvas, choosing,
 * moving, turning, wires, labels, the inspector's place, keys, the picture) is the editor's.
 * `K` is what the palette offers, `S` what a part shows of the simulation.
 */
export interface PartKit<E extends Placeable, K extends string, S = undefined> {
  /** Which editors share a clipboard: parts copied in one paste only into one of the same `id`. */
  id: string;
  /** The kinds the palette offers, in order. */
  palette: K[];
  paletteLabel(kind: K, t: Translate): string;
  /** A palette entry's picture, a whole `<svg>`. */
  paletteIcon(kind: K): ReactNode;
  /** How many grid steps apart parts put in from the palette go. */
  spacing: number;
  /** Whether another part of `kind` may go in (there are two terminals at most). */
  canAdd?(elements: E[], kind: K): boolean;
  newPart(elements: E[], kind: K, at: Point): E;
  newWire(elements: E[], from: Point, to: Point): E;
  /** A copied part made new to be pasted into `into`: a fresh id, a free name; null when it may not go in. */
  pasted(part: E, into: E[]): E | null;

  /** A part's pins at their places on the canvas; both ends of a wire. */
  pins(element: E): Point[];
  /** Whether a press on a part's pin moves the part (a ground) instead of drawing a wire. */
  pinMoves?(element: E): boolean;
  /** The wires that join whatever sits in their middle. */
  joining(elements: E[]): Segment[];
  /** Whether a wire may start in the middle of `wire`, making a T-junction. */
  wireJoins(wire: E): boolean;
  /** The pins and wire ends that count for junction dots; every pin when left out. */
  dotEnds?(elements: E[]): Point[];
  /** A part's rotation after a quarter turn. */
  turned?(rotation: number): number;

  /** A part's drawn body on the canvas: where a leader starts and the inspector goes beside. */
  body(element: E): Rect;
  /** Where a part is grabbed, in its own coordinates. */
  hit(element: E): Rect;
  /** What labels keep clear of: body, leads and pins. */
  obstacles(element: E): Rect[];
  /** A part in its own coordinates, unturned. */
  symbol(element: E, state?: S): ReactNode;
  /** Drawn over a part at its place, unturned, such as a terminal's name. */
  decoration?(element: E): ReactNode;
  /** Whether a press on the part only does the editor's `onTap` (a switch); a double click opens it. */
  tapOnly?(element: E): boolean;

  /** A mode's own keys, tried before the editor's. */
  keys?(context: KitContext<E>): KeyBinding[];

  /** The mode's own CSS of an exported picture, after the shared one; no export without it. */
  pictureStyle?(colors: PictureColors): string;
}
