/**
 * Whether the analysis input is a list of points, `(0, 1), (1, 3)` or `(0|1); (1|3)`, so the
 * view asks for a curve through them instead of a curve sketch. The engine reads the points
 * itself; this only decides which call to make, like `spline::is_point_list` in the core.
 */
export function isPointList(latex: string): boolean {
  let text = latex;
  for (const spacing of ['\\left', '\\right', '\\qquad', '\\quad', '\\,', '\\;', '\\ ', '~']) text = text.split(spacing).join(' ');
  text = text.split('\\mid').join('|').split('\\vert').join('|').trim();
  if (!text.startsWith('(') || !text.endsWith(')')) return false;
  const insides: string[] = [];
  let depth = 0;
  let start = 0;
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (c === '(') {
      if (depth === 0) start = i + 1;
      depth++;
    } else if (c === ')') {
      depth--;
      if (depth < 0) return false;
      if (depth === 0) insides.push(text.slice(start, i));
    } else if (depth === 0 && !/[,; ]/.test(c)) {
      return false;
    }
  }
  if (depth !== 0 || insides.length < 2) return false;
  return insides.every((inside) => {
    const separator = ['|', ';', ','].find((s) => inside.includes(s));
    if (!separator) return false;
    const [x, ...rest] = inside.split(separator);
    const y = rest.join(separator);
    return x.trim() !== '' && y.trim() !== '' && !y.includes(separator);
  });
}
