// Shared library state: the sidebar, the grid and the recipe page all read from here,
// so a move, rename or import shows up everywhere at once.
import { listRecipes, listFolders, listTags, moveRecipe, searchRecipes, type RecipeSummary, type Folder, type Tag } from "./api";

export type Scope = { kind: "all" } | { kind: "unfiled" } | { kind: "folder"; id: number } | { kind: "tag"; name: string };

/** Data type used when dragging a recipe inside the app (keeps it apart from dragged links). */
export const RECIPE_DRAG = "application/x-recipe-id";

export const lib = $state({
  recipes: [] as RecipeSummary[],
  folders: [] as Folder[],
  tags: [] as Tag[],
  scope: { kind: "all" } as Scope,
  loaded: false,
});

export async function refresh() {
  const [recipes, folders, tags] = await Promise.all([listRecipes(), listFolders(), listTags()]);
  lib.recipes = recipes;
  lib.folders = folders;
  lib.tags = tags;
  lib.loaded = true;
  // The open folder may have been deleted.
  const s = lib.scope;
  if (s.kind === "folder" && !folders.some((f) => f.id === s.id)) lib.scope = { kind: "all" };
  if (s.kind === "tag" && !tags.some((t) => t.name.toLowerCase() === s.name.toLowerCase())) lib.scope = { kind: "all" };
}

export function inScope(r: RecipeSummary, s: Scope) {
  if (s.kind === "all") return true;
  if (s.kind === "unfiled") return r.folder_id === null;
  if (s.kind === "tag") return r.tags.some((t) => t.toLowerCase() === s.name.toLowerCase());
  return r.folder_id === s.id;
}

export function scopeName(s: Scope) {
  if (s.kind === "all") return "All recipes";
  if (s.kind === "unfiled") return "Not in a folder";
  if (s.kind === "tag") return `#${s.name}`;
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

// ---------- Search ----------

export const search = $state({
  query: "",
  advanced: false,
  tags: [] as string[], // recipe must have all of these
  author: "",
  site: "",
  maxTime: 0, // minutes, 0 = any
  folder: "any" as "any" | "none" | number,
  /** Ids from the text search, in ranked order; null when there's no text query. */
  hits: null as number[] | null,
});

export function searchActive() {
  return (
    search.query.trim() !== "" ||
    search.tags.length > 0 ||
    search.author !== "" ||
    search.site !== "" ||
    search.maxTime > 0 ||
    search.folder !== "any"
  );
}

let timer: ReturnType<typeof setTimeout> | undefined;
let seq = 0;
/** Runs the text search shortly after typing stops. */
export function queueSearch() {
  clearTimeout(timer);
  const q = search.query.trim();
  if (!q) {
    search.hits = null;
    return;
  }
  timer = setTimeout(async () => {
    const mine = ++seq;
    const ids = await searchRecipes(q);
    if (mine === seq) search.hits = ids; // ignore answers to older queries
  }, 200);
}

export function clearSearch() {
  search.query = "";
  search.tags = [];
  search.author = "";
  search.site = "";
  search.maxTime = 0;
  search.folder = "any";
  search.hits = null;
}

/** Recipes matching the text search and every advanced filter, best text matches first. */
export function searchResults(): RecipeSummary[] {
  let list = lib.recipes;
  if (search.hits) {
    const byId = new Map(list.map((r) => [r.id, r]));
    list = search.hits.map((id) => byId.get(id)).filter((r): r is RecipeSummary => !!r);
  }
  const lower = (s: string) => s.toLowerCase();
  return list.filter(
    (r) =>
      search.tags.every((t) => r.tags.some((x) => lower(x) === lower(t))) &&
      (!search.author || r.author === search.author) &&
      (!search.site || r.site_name === search.site) &&
      (!search.maxTime || (r.total_min !== null && r.total_min <= search.maxTime)) &&
      (search.folder === "any" || (search.folder === "none" ? r.folder_id === null : r.folder_id === search.folder)),
  );
}
