import type { GeometryObject, GeometryRequest, GeometryTask } from './api';

/** The input of the Geometry mode: the scene's objects and the task on them, as the engine takes them. */

/** What the page holds; `App` keeps it, so it survives a switch to another mode and back. */
export interface GeometryInput {
  objects: GeometryObject[];
  task: GeometryTask | null;
}

export const EMPTY_GEOMETRY: GeometryInput = { objects: [], task: null };

/** The scene as a request, or null while it is empty. */
export function geometryRequest(input: GeometryInput): GeometryRequest | null {
  if (!input.objects.length && !input.task) return null;
  return { scene: { objects: input.objects }, task: input.task };
}

/** The input a session saved, or the empty one. */
export function geometryInput(request: GeometryRequest | null | undefined): GeometryInput {
  if (!request) return EMPTY_GEOMETRY;
  return { objects: request.scene?.objects ?? [], task: request.task ?? null };
}
