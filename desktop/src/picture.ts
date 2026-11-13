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
