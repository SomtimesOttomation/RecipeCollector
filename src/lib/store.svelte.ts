// Shared library state: the sidebar, the grid and the recipe page all read from here,
// so a move, rename or import shows up everywhere at once.
import { listRecipes, listFolders, moveRecipe, type RecipeSummary, type Folder } from "./api";

export type Scope = { kind: "all" } | { kind: "unfiled" } | { kind: "folder"; id: number };

/** Data type used when dragging a recipe inside the app (keeps it apart from dragged links). */
export const RECIPE_DRAG = "application/x-recipe-id";

export const lib = $state({
  recipes: [] as RecipeSummary[],
  folders: [] as Folder[],
  scope: { kind: "all" } as Scope,
  loaded: false,
});

export async function refresh() {
  const [recipes, folders] = await Promise.all([listRecipes(), listFolders()]);
  lib.recipes = recipes;
  lib.folders = folders;
  lib.loaded = true;
  // The open folder may have been deleted.
  const s = lib.scope;
  if (s.kind === "folder" && !folders.some((f) => f.id === s.id)) lib.scope = { kind: "all" };
}

export function inScope(r: RecipeSummary, s: Scope) {
  if (s.kind === "all") return true;
  if (s.kind === "unfiled") return r.folder_id === null;
  return r.folder_id === s.id;
}

export function scopeName(s: Scope) {
  if (s.kind === "all") return "All recipes";
  if (s.kind === "unfiled") return "Not in a folder";
  return lib.folders.find((f) => f.id === s.id)?.name ?? "Folder";
}

/** The folder new imports should land in: the one being viewed, if any. */
export function currentFolderId(): number | null {
  return lib.scope.kind === "folder" ? lib.scope.id : null;
}

export async function moveTo(recipeId: number, folderId: number | null) {
  await moveRecipe(recipeId, folderId);
  await refresh();
}

export function draggedRecipe(e: DragEvent): number | null {
  const raw = e.dataTransfer?.getData(RECIPE_DRAG);
  const id = raw ? parseInt(raw, 10) : NaN;
  return Number.isFinite(id) ? id : null;
}

export function isRecipeDrag(e: DragEvent) {
  return !!e.dataTransfer && Array.from(e.dataTransfer.types).includes(RECIPE_DRAG);
}
