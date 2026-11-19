import { placeLabels, type Placement } from './schematic/labels';

/** How far a label sits from its anchor, in pixels. */
export const LABEL_GAP = 6;
/** The size of a point's marker, which a label must not cover. */
const MARKER = 8;
/** A name's estimated size at the view's 13 px italic: a letter's width, the line's height. */
const LETTER = 8;
const LINE = 14;

export interface ViewLabel {
  id: string;
  /** The anchor in pixels. */
  x: number;
  y: number;
  /** The name as written, `P_1`; the index is set smaller. */
  text: string;
}

/** The estimated width of a name in pixels: its letters, the index smaller. */
export function labelWidth(text: string): number {
  const [base, index = ''] = text.replace(/\\/g, '').split('_', 2);
  return base.length * LETTER + index.length * LETTER * 0.75 + (text.includes('_') ? 1 : 0);
}

/**
 * The names in the 3D view placed so that none covers another or a point: each one above and to
 * the right of its anchor if that is free, otherwise in the next free spot around it. Placed in
 * the order given; `previous` keeps a label where it was while the view turns.
 */
export function spreadLabels(labels: ViewLabel[], markers: { x: number; y: number }[], previous?: Map<string, Placement>): Map<string, Placement> {
  return placeLabels(
    labels.map((label) => ({
      id: label.id,
      anchor: { x: label.x, y: label.y },
      sides: ['above', 'below', 'right', 'left'],
      gap: LABEL_GAP,
      corner: true,
      w: labelWidth(label.text),
      h: LINE,
    })),
    // Every anchor is kept free as a point's marker is, so no name sits on another's anchor.
    [...markers, ...labels].map((m) => ({ x: m.x - MARKER / 2, y: m.y - MARKER / 2, w: MARKER, h: MARKER })),
    previous,
  );
}
