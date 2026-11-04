import type { GeometryEntry, GeometryObject, GeometryRequest, GeometryShape, GeometryTask } from './api';

/** The input of the Geometry mode: the scene's objects and the task on them, as the engine takes them. */

/** What the page holds; `App` keeps it, so it survives a switch to another mode and back. */
export interface GeometryInput {
  objects: GeometryObject[];
  task: GeometryTask | null;
}

export const EMPTY_GEOMETRY: GeometryInput = { objects: [], task: null };

/** The kinds of object a form can make. */
export type FormKind = 'point' | 'vector' | 'line' | 'plane' | 'sphere' | 'circle';

/** How an object of a form is given: the shape kinds of the engine, without the typed line. */
export type Variant = Exclude<GeometryShape['kind'], 'typed'>;

/** The ways each kind of object can be given, the first one the default. */
export const VARIANTS: Record<FormKind, Variant[]> = {
  point: ['point'],
  vector: ['vector'],
  line: ['line', 'lineThrough'],
  plane: ['plane', 'planeNormal', 'planeCoordinate', 'planeThrough'],
  sphere: ['sphere', 'sphereThrough', 'sphereFour'],
  circle: ['circle', 'circleCut'],
};

/** The kind of object a shape gives, or null for a typed line. */
export function formKindOf(kind: GeometryShape['kind']): FormKind | null {
  for (const [form, variants] of Object.entries(VARIANTS) as [FormKind, Variant[]][]) if (variants.includes(kind as Variant)) return form;
  return null;
}

/** Three empty coordinate cells. */
const cells = (): string[] => ['', '', ''];

/** A new, empty object given as `variant`, named `name`. */
export function emptyShape(variant: Variant, name: string): GeometryObject {
  switch (variant) {
    case 'point':
      return { name, kind: variant, at: cells() };
    case 'vector':
      return { name, kind: variant, value: cells() };
    case 'line':
      return { name, kind: variant, support: cells(), direction: cells() };
    case 'lineThrough':
      return { name, kind: variant, a: cells(), b: cells() };
    case 'plane':
      return { name, kind: variant, support: cells(), u: cells(), v: cells() };
    case 'planeNormal':
      return { name, kind: variant, support: cells(), normal: cells() };
    case 'planeCoordinate':
      return { name, kind: variant, a: '', b: '', c: '', d: '' };
    case 'planeThrough':
      return { name, kind: variant, a: cells(), b: cells(), c: cells() };
    case 'sphere':
      return { name, kind: variant, centre: cells(), radius: '' };
    case 'sphereThrough':
      return { name, kind: variant, centre: cells(), point: cells() };
    case 'sphereFour':
      return { name, kind: variant, a: cells(), b: cells(), c: cells(), d: cells() };
    case 'circle':
      return { name, kind: variant, centre: cells(), radius: '', normal: cells() };
    case 'circleCut':
      return { name, kind: variant, sphere: '', plane: '' };
  }
}

/** A new, empty object of `kind`, given the default way. */
export function newObject(kind: FormKind, name: string): GeometryObject {
  return emptyShape(VARIANTS[kind][0], name);
}

/** The point an object is given by first: its support, its first point, its centre. */
function firstPoint(object: GeometryObject): GeometryEntry | null {
  const fields = object as unknown as Record<string, unknown>;
  for (const key of ['support', 'centre', 'at']) if (Array.isArray(fields[key]) || typeof fields[key] === 'string') return fields[key] as GeometryEntry;
  // The first of several points; a coordinate form's `a` is a number, not a point.
  if (object.kind !== 'planeCoordinate' && fields.a !== undefined) return fields.a as GeometryEntry;
  return null;
}

/**
 * The object given another way: the fields of the new way start empty, except the point it is
 * given by first (support, first point or centre), which carries over.
 */
export function switchVariant(object: GeometryObject, variant: Variant): GeometryObject {
  if (object.kind === variant) return object;
  const next = emptyShape(variant, object.name ?? '') as unknown as Record<string, unknown>;
  const point = firstPoint(object);
  if (point) for (const key of ['support', 'centre', 'a']) if (key in next && Array.isArray(next[key])) {
    next[key] = point;
    break;
  }
  return next as unknown as GeometryObject;
}

/** As in school: A, B for points, a, b for vectors, g, h for lines, E, F for planes, K, L for spheres and k for circles. */
const LETTERS: Record<FormKind, string[]> = {
  point: 'ABCDPQRSTUVW'.split(''),
  vector: 'abcduvw'.split(''),
  line: 'gh'.split(''),
  plane: 'EFGH'.split(''),
  sphere: 'KLM'.split(''),
  circle: 'k'.split(''),
};
const BASES: Record<FormKind, string> = { point: 'P', vector: 'v', line: 'g', plane: 'E', sphere: 'K', circle: 'k' };

/** The first name of its kind that the scene does not use yet, then `P_1`, `P_2`, … */
export function freeName(kind: FormKind, objects: GeometryObject[]): string {
  const taken = new Set(objects.map((object) => (object.name ?? '').replace(/[{}\s]/g, '')));
  const letter = LETTERS[kind].find((name) => !taken.has(name));
  if (letter) return letter;
  const base = BASES[kind];
  for (let k = 1; ; k++) if (!taken.has(`${base}_${k}`)) return `${base}_${k}`;
}

/** Cells with an empty z are a 2D entry, so that a scene of 2D inputs gives 2D results. */
function entry(value: GeometryEntry): GeometryEntry {
  return Array.isArray(value) && value.length === 3 && !value[2].trim() ? value.slice(0, 2) : value;
}

/** The object as the engine takes it: its point and vector entries with an empty z dropped, an empty normal left out. */
function shapeRequest(object: GeometryObject): GeometryObject {
  const out = mapEntries(object, entry);
  // A circle without a normal lies in the x-y plane.
  if (out.kind === 'circle' && Array.isArray(out.normal) && out.normal.every((cell) => !cell.trim())) return { ...out, normal: null };
  return out;
}

/** The object with `f` applied to each of its coordinate entries. */
function mapEntries(object: GeometryObject, f: (value: GeometryEntry) => GeometryEntry): GeometryObject {
  const out: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(object)) out[key] = Array.isArray(value) ? f(value as GeometryEntry) : value;
  return out as GeometryShape & { name?: string };
}

/** A saved 2D entry gets its empty z cell back for the form. */
const padded = (value: GeometryEntry): GeometryEntry => (Array.isArray(value) && value.length === 2 ? [...value, ''] : value);

/** A saved object as the form shows it: 2D entries padded, a circle without normal with empty normal cells. */
function formObject(object: GeometryObject): GeometryObject {
  const out = mapEntries(object, padded);
  return out.kind === 'circle' && !out.normal ? { ...out, normal: ['', '', ''] } : out;
}

/** The scene as a request, or null while it is empty. */
export function geometryRequest(input: GeometryInput): GeometryRequest | null {
  if (!input.objects.length && !input.task) return null;
  return { scene: { objects: input.objects.map(shapeRequest) }, task: input.task };
}

/** `|` as MathLive may write it between coordinates, as the engine reads it. */
export function plainBars(latex: string): string {
  return latex.replace(/\\middle\s*\|/g, '|').replace(/\\mid\b/g, '|');
}

/** A name without braces and spaces: `P_{1}` is `P_1`. */
const plain = (name: string) => name.replace(/[{}\s]/g, '');

/**
 * The key of the object a typed line names, as the scene tells objects apart: `vec:a` for
 * `\vec{a} = …`, `P_1` for `P_{1}(1|2)`, `g` for `g: X = …`, `E` for `E: x = 1`. Null if the line
 * starts with no name.
 */
export function typedKey(latex: string): string | null {
  const text = latex.trim();
  const vector = /^\\vec\s*(?:\{\s*([A-Za-z](?:_\{?[A-Za-z0-9]+\}?)?)\s*\}|([A-Za-z]))\s*=/.exec(text);
  if (vector) return `vec:${plain(vector[1] ?? vector[2])}`;
  const named = /^([A-Za-z](?:_\{?[A-Za-z0-9]+\}?)?)\s*(?:\(|\\left\s*\(|:|=)/.exec(text);
  return named ? plain(named[1]) : null;
}

/** The key of an object of the scene, in the namespaces of `typedKey`. */
export function objectKey(object: GeometryObject): string | null {
  if (object.kind === 'typed') return typedKey(object.latex);
  const name = plain(object.name ?? '').replace(/^\\vec/, '');
  if (!name) return null;
  return object.kind === 'vector' ? `vec:${name}` : name;
}

/**
 * The scene with a typed line: it replaces the object of the same name where that stands, so
 * that later objects built on it still come after it; a new name goes at the end.
 */
export function withTyped(objects: GeometryObject[], latex: string): GeometryObject[] {
  const typed: GeometryObject = { kind: 'typed', latex: plainBars(latex.trim()) };
  const key = typedKey(typed.latex);
  const index = key === null ? -1 : objects.findIndex((object) => objectKey(object) === key);
  return index < 0 ? [...objects, typed] : objects.map((object, i) => (i === index ? typed : object));
}

/** The input a session saved, or the empty one. */
export function geometryInput(request: GeometryRequest | null | undefined): GeometryInput {
  if (!request) return EMPTY_GEOMETRY;
  return { objects: (request.scene?.objects ?? []).map(formObject), task: request.task ?? null };
}
