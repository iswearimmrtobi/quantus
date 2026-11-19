import { renderToStaticMarkup } from 'react-dom/server';
import { boundsOf, junctionDotsOf, pointKey, wireEnds, type Placeable, type Point, type Rect } from './geometry';
import type { PartKit } from './kit';
import { leaderOf, placeLabels, type CanvasLabel, type Placement } from './labels';

/**
 * Pictures of a drawing for export (design/digital-logic.md §10.5): a standalone SVG, and a PNG
 * rendered from it.
 */

/** A standalone SVG document and its size in px. */
export interface Picture {
  svg: string;
  width: number;
  height: number;
}

/** The SVG as bytes, for a file. */
export const svgBytes = (picture: Picture): Uint8Array => new TextEncoder().encode(picture.svg);

/** The SVG rendered at `scale` times its size as a PNG. */
export function pngBytes(picture: Picture, scale = 2): Promise<Uint8Array> {
  return new Promise((resolve, reject) => {
    const image = new Image();
    image.onload = () => {
      const canvas = document.createElement('canvas');
      canvas.width = Math.ceil(picture.width * scale);
      canvas.height = Math.ceil(picture.height * scale);
      const context = canvas.getContext('2d');
      if (!context) return reject(new Error('no canvas'));
      context.drawImage(image, 0, 0, canvas.width, canvas.height);
      canvas.toBlob((blob) => {
        if (!blob) return reject(new Error('no PNG'));
        blob.arrayBuffer().then((buffer) => resolve(new Uint8Array(buffer)), reject);
      }, 'image/png');
    };
    image.onerror = () => reject(new Error('the picture could not be drawn'));
    image.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(picture.svg)}`;
  });
}

/** The colours of a picture: ink, the paper (also the fill of bodies), a grey for knobs and one for faint labels. */
export interface PictureColors {
  ink: string;
  paper: string;
  muted: string;
  faint: string;
  /** A net at 1 and a lit lamp, for a picture that shows a simulation. */
  on?: string;
  lit?: string;
}

/** A simulation drawn into a picture: a class per wire (`on`, `off`) and each part's state for its symbol. */
export interface PictureState<E, S> {
  wireClass(wire: E): string | undefined;
  partState(element: E): S | undefined;
}

/** An exported picture: the light theme's colours, black on white. */
export const EXPORT_COLORS: PictureColors = { ink: '#1b1a22', paper: '#ffffff', muted: '#6b6878', faint: '#8a8698' };

/** The app's colours as the current theme sets them, for a picture shown in the app. */
export function appColors(): PictureColors {
  const style = getComputedStyle(document.documentElement);
  const read = (name: string, fallback: string) => style.getPropertyValue(name).trim() || fallback;
  return {
    ink: read('--text', EXPORT_COLORS.ink),
    paper: read('--background', EXPORT_COLORS.paper),
    muted: read('--text-muted', EXPORT_COLORS.muted),
    faint: read('--text-faint', EXPORT_COLORS.faint),
    on: read('--point-zero', '#3fb37f'),
    lit: read('--point-saddle', '#e0a030'),
  };
}

/** What every picture draws alike: wires, junction dots and the leaders of moved labels. */
const BASE_STYLE = ({ ink, faint }: PictureColors) => `
.symbol, .wire { fill: none; stroke: ${ink}; stroke-width: 2; stroke-linecap: round; stroke-linejoin: round; }
.junction-dot { fill: ${ink}; }
.label-leader { stroke: ${faint}; stroke-width: 1; stroke-dasharray: 2 3; }
`;

/**
 * A drawing as a standalone SVG (§10.5): the parts, wires, junctions and the labels that have an
 * `svg` form, where the editor shows them, without the grid, the pins, a selection or the
 * simulation; on `paper` unless that is false.
 */
export function schematicPicture<E extends Placeable, K extends string, S>(
  kit: PartKit<E, K, S>,
  elements: E[],
  labels: CanvasLabel[],
  placements: Map<string, Placement>,
  colors: PictureColors = EXPORT_COLORS,
  paper = true,
  state?: PictureState<E, S>,
): Picture {
  const margin = 16;
  const drawn = labels.filter((label) => label.svg);
  const boxOf = (label: CanvasLabel): Rect => placements.get(label.id) ?? { x: label.anchor.x, y: label.anchor.y, w: 0, h: 0 };
  const box = boundsOf([...elements.flatMap(kit.obstacles), ...drawn.map(boxOf)]);
  const [x, y, width, height] = [box.x - margin, box.y - margin, Math.ceil(box.w + 2 * margin), Math.ceil(box.h + 2 * margin)];
  const dots = junctionDotsOf(kit.dotEnds?.(elements) ?? elements.flatMap(kit.pins), kit.joining(elements));
  const body = renderToStaticMarkup(
    <>
      {elements.map((element) => {
        if (element.kind === 'wire') {
          const [a, b] = wireEnds(element);
          const value = state?.wireClass(element);
          return <line key={element.id} className={value ? `wire ${value}` : 'wire'} x1={a.x} y1={a.y} x2={b.x} y2={b.y} />;
        }
        return (
          <g key={element.id}>
            <g className="symbol" transform={`translate(${element.x} ${element.y}) rotate(${element.rotation * 90})`}>
              {kit.symbol(element, state?.partState(element))}
            </g>
            {kit.decoration?.(element)}
          </g>
        );
      })}
      {dots.map((point: Point) => (
        <circle key={pointKey(point)} className="junction-dot" cx={point.x} cy={point.y} r="3.5" />
      ))}
      {drawn.map((label) => {
        const placement = placements.get(label.id);
        const at = boxOf(label);
        const leader = placement?.displaced && leaderOf(label, placement);
        return (
          <g key={label.id}>
            {leader && <line className="label-leader" x1={leader[0].x} y1={leader[0].y} x2={leader[1].x} y2={leader[1].y} />}
            <g transform={`translate(${at.x + at.w / 2} ${at.y + at.h / 2})`}>{label.svg}</g>
          </g>
        );
      })}
    </>,
  );
  const style = BASE_STYLE(colors) + (kit.pictureStyle?.(colors) ?? '');
  const svg =
    `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="${x} ${y} ${width} ${height}">` +
    `<style>${style}</style>${paper ? `<rect x="${x}" y="${y}" width="${width}" height="${height}" fill="${colors.paper}"/>` : ''}${body}</svg>`;
  return { svg, width, height };
}

/**
 * A drawing as a picture without an editor (the Logic Gates card): the labels' sizes guessed from
 * their text instead of measured, in the app's colours (`theme: 'app'`) or black on white.
 */
export function guessedPicture<E extends Placeable, K extends string, S>(
  kit: PartKit<E, K, S>,
  elements: E[],
  labels: CanvasLabel[],
  theme: 'app' | 'export',
  state?: PictureState<E, S>,
): Picture {
  const requests = labels.filter((label) => label.guess).map((label) => ({ ...label, ...label.guess! }));
  const placements = placeLabels(requests, elements.flatMap(kit.obstacles));
  return schematicPicture(kit, elements, labels, placements, theme === 'app' ? appColors() : EXPORT_COLORS, theme === 'export', state);
}
