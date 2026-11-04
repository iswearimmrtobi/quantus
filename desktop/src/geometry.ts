import type { GeometryEntry, GeometryObject, GeometryRequest, GeometryShape, GeometryTask } from './api';

/** The input of the Geometry mode: the scene's objects and the task on them, as the engine takes them. */

/** What the page holds; `App` keeps it, so it survives a switch to another mode and back. */
export interface GeometryInput {
  objects: GeometryObject[];
  task: GeometryTask | null;
}

export const EMPTY_GEOMETRY: GeometryInput = { objects: [], task: null };

/** The kinds of object a form can make. */
export type FormKind = 'point' | 'vector';

/** Three empty coordinate cells. */
const cells = (): string[] => ['', '', ''];

/** A new, empty object of `kind` named `name`. */
export function newObject(kind: FormKind, name: string): GeometryObject {
  switch (kind) {
    case 'point':
      return { name, kind, at: cells() };
    case 'vector':
      return { name, kind, value: cells() };
  }
}

/** Upper case for points (A, B, …), lower case for vectors (a, b, …), as in school; E, K and g stay free for planes, spheres and lines. */
const LETTERS: Record<FormKind, string[]> = {
  point: 'ABCDFGHIJLMNOPQRSTUVW'.split(''),
  vector: 'abcdefhijklmnopqrstuvw'.split(''),
};

/** The first name of its kind that the scene does not use yet, then `P_1`, `P_2`, … */
export function freeName(kind: FormKind, objects: GeometryObject[]): string {
  const taken = new Set(objects.map((object) => (object.name ?? '').replace(/[{}\s]/g, '')));
  const letter = LETTERS[kind].find((name) => !taken.has(name));
  if (letter) return letter;
  const base = kind === 'point' ? 'P' : 'v';
  for (let k = 1; ; k++) if (!taken.has(`${base}_${k}`)) return `${base}_${k}`;
}

/** Cells with an empty z are a 2D entry, so that a scene of 2D inputs gives 2D results. */
function entry(value: GeometryEntry): GeometryEntry {
  return Array.isArray(value) && value.length === 3 && !value[2].trim() ? value.slice(0, 2) : value;
}

/** The object as the engine takes it: its point and vector entries with an empty z dropped. */
function shapeRequest(object: GeometryObject): GeometryObject {
  return mapEntries(object, entry);
}

/** The object with `f` applied to each of its coordinate entries. */
function mapEntries(object: GeometryObject, f: (value: GeometryEntry) => GeometryEntry): GeometryObject {
  const out: Record<string, unknown> = {};
  for (const [key, value] of Object.entries(object)) out[key] = Array.isArray(value) ? f(value as GeometryEntry) : value;
  return out as GeometryShape & { name?: string };
}

/** A saved 2D entry gets its empty z cell back for the form. */
const padded = (value: GeometryEntry): GeometryEntry => (Array.isArray(value) && value.length === 2 ? [...value, ''] : value);

/** The scene as a request, or null while it is empty. */
export function geometryRequest(input: GeometryInput): GeometryRequest | null {
  if (!input.objects.length && !input.task) return null;
  return { scene: { objects: input.objects.map(shapeRequest) }, task: input.task };
}

/** The input a session saved, or the empty one. */
export function geometryInput(request: GeometryRequest | null | undefined): GeometryInput {
  if (!request) return EMPTY_GEOMETRY;
  return { objects: (request.scene?.objects ?? []).map((object) => mapEntries(object, padded)), task: request.task ?? null };
}
