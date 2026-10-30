// Thin wrapper over the Rust commands. Falls back to sample data when the UI
// runs in a plain browser (npm run dev) so the layout can be worked on anywhere.
import { invoke, convertFileSrc } from "@tauri-apps/api/core";

export interface LibraryInfo {
  path: string;
  recipe_count: number;
  schema_version: number;
}

export interface RecipeSummary {
  id: number;
  title: string;
  total_min: number | null;
  site_name: string | null;
  hero_image: string | null;
}

export interface Section {
  title: string | null;
  items: string[];
}

export interface ImageInfo {
  id: number;
  path: string;
  is_hero: boolean;
}

export interface Recipe {
  id: number;
  title: string;
  description: string | null;
  source_url: string | null;
  site_name: string | null;
  author: string | null;
  servings: string | null;
  prep_min: number | null;
  cook_min: number | null;
  total_min: number | null;
  notes: string | null;
  ingredients: Section[];
  steps: Section[];
  images: ImageInfo[];
}

export const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const sample: Recipe = {
  id: 1,
  title: "Caldo Verde",
  description: "Portuguese potato and kale soup.",
  source_url: "https://example.com/caldo-verde",
  site_name: "example.com",
  author: null,
  servings: "4 servings",
  prep_min: 15,
  cook_min: 30,
  total_min: 45,
  notes: null,
  ingredients: [{ title: null, items: ["1 lb potatoes", "4 cups kale, thinly sliced", "1 chouriço"] }],
  steps: [{ title: null, items: ["Boil the potatoes until soft.", "Blend, add the kale and simmer 5 minutes.", "Serve with chouriço slices."] }],
  images: [],
};

/** URL the webview can load for a photo saved in the library. */
export function photoUrl(path: string | null): string | null {
  if (!path) return null;
  return inTauri ? convertFileSrc(path) : path;
}

export async function libraryInfo(): Promise<LibraryInfo> {
  if (!inTauri) return { path: "~/Documents/Recipe Collector", recipe_count: 1, schema_version: 2 };
  return invoke<LibraryInfo>("library_info");
}

export async function listRecipes(): Promise<RecipeSummary[]> {
  if (!inTauri) return [{ id: 1, title: sample.title, total_min: 45, site_name: "example.com", hero_image: null }];
  return invoke<RecipeSummary[]>("list_recipes");
}

export async function revealLibrary(): Promise<void> {
  if (!inTauri) return;
  return invoke("reveal_library");
}

export async function importRecipe(url: string): Promise<number> {
  if (!inTauri) {
    await new Promise((r) => setTimeout(r, 800));
    return 1;
  }
  return invoke<number>("import_recipe", { url });
}

export async function getRecipe(id: number): Promise<Recipe> {
  if (!inTauri) return structuredClone(sample);
  return invoke<Recipe>("get_recipe", { id });
}

export async function updateRecipe(recipe: Recipe): Promise<void> {
  if (!inTauri) return;
  return invoke("update_recipe", { recipe });
}

export async function deleteRecipe(id: number): Promise<void> {
  if (!inTauri) return;
  return invoke("delete_recipe", { id });
}

export async function addImageFromFile(recipeId: number, path: string, makeHero: boolean): Promise<ImageInfo | null> {
  if (!inTauri) return null;
  return invoke<ImageInfo>("add_image_from_file", { recipeId, path, makeHero });
}

export async function setHero(imageId: number): Promise<void> {
  if (!inTauri) return;
  return invoke("set_hero", { imageId });
}

export async function removeImage(imageId: number): Promise<void> {
  if (!inTauri) return;
  return invoke("remove_image", { imageId });
}

/** Lets the user pick a photo file; returns its path or null. */
export async function pickPhoto(): Promise<string | null> {
  if (!inTauri) return null;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const picked = await open({
    multiple: false,
    filters: [{ name: "Photos", extensions: ["jpg", "jpeg", "png", "webp", "gif"] }],
  });
  return typeof picked === "string" ? picked : null;
}

export async function confirmDelete(title: string): Promise<boolean> {
  if (!inTauri) return window.confirm(`Delete "${title}"?`);
  const { ask } = await import("@tauri-apps/plugin-dialog");
  return ask(`Delete "${title}" and its photos? This can't be undone.`, { title: "Delete recipe", kind: "warning" });
}

export async function openLink(url: string): Promise<void> {
  if (!inTauri) {
    window.open(url, "_blank");
    return;
  }
  const { openUrl } = await import("@tauri-apps/plugin-opener");
  await openUrl(url);
}
