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
  author: string | null;
  hero_image: string | null;
  folder_id: number | null;
  tags: string[];
}

export interface Tag {
  id: number;
  name: string;
  recipe_count: number;
}

export interface Folder {
  id: number;
  name: string;
  recipe_count: number;
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
  video_url: string | null;
  tags: string[];
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
  video_url: null,
  tags: ["Soup", "Portuguese"],
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
  if (!inTauri) return [{ id: 1, title: sample.title, total_min: 45, site_name: "example.com", author: null, hero_image: null, folder_id: null, tags: ["Soup", "Portuguese"] }];
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

/** Opens the page in the in-app browser so the user can import it from there. */
export async function openImportBrowser(url: string): Promise<void> {
  if (!inTauri) {
    window.open(url, "_blank");
    return;
  }
  return invoke("open_import_browser", { url });
}

/** Calls back with the new recipe id when an import finishes in the in-app browser. */
export async function onBrowserImport(handler: (id: number) => void): Promise<() => void> {
  if (!inTauri) return () => {};
  const { listen } = await import("@tauri-apps/api/event");
  return listen<number>("recipe-imported", (e) => handler(e.payload));
}

/** The 11-character video id of a YouTube link, or null. */
export function youtubeId(url: string | null): string | null {
  if (!url) return null;
  const m = url.match(/(?:youtube(?:-nocookie)?\.com\/(?:watch\?(?:.*&)?v=|embed\/|shorts\/|live\/)|youtu\.be\/)([A-Za-z0-9_-]{11})/);
  return m ? m[1] : null;
}

/** Opens the system print dialog for the current page. */
export async function printPage(): Promise<void> {
  if (!inTauri) {
    window.print();
    return;
  }
  return invoke("print_page");
}

export type UpdateStatus =
  | { state: "up_to_date"; current: string; latest: string }
  | {
      state: "available";
      current: string;
      latest: string;
      notes: string;
      page_url: string;
      installer_url: string | null;
      checksum_url: string | null;
    }
  | { state: "unavailable"; current: string; reason: string };

export async function appVersion(): Promise<string> {
  if (!inTauri) return "0.1.0";
  return invoke<string>("app_version");
}

export async function checkForUpdate(): Promise<UpdateStatus> {
  if (!inTauri) return { state: "unavailable", current: "0.1.0", reason: "Updates are checked in the installed app." };
  return invoke<UpdateStatus>("check_for_update");
}

export async function installUpdate(installerUrl: string, checksumUrl: string | null): Promise<void> {
  if (!inTauri) return;
  return invoke("install_update", { installerUrl, checksumUrl });
}

const AUTO_KEY = "rc.autoUpdateCheck";
export function autoCheckEnabled(): boolean {
  try {
    return localStorage.getItem(AUTO_KEY) !== "off";
  } catch {
    return true;
  }
}
export function setAutoCheck(on: boolean) {
  try {
    localStorage.setItem(AUTO_KEY, on ? "on" : "off");
  } catch {}
}

export const isMacPlatform = typeof navigator !== "undefined" && /Mac/.test(navigator.userAgent);

// ---------- Folders ----------

export async function listFolders(): Promise<Folder[]> {
  if (!inTauri) return [];
  return invoke<Folder[]>("list_folders");
}

export async function createFolder(name: string): Promise<Folder | null> {
  if (!inTauri) return null;
  return invoke<Folder>("create_folder", { name });
}

export async function renameFolder(id: number, name: string): Promise<void> {
  if (!inTauri) return;
  return invoke("rename_folder", { id, name });
}

export async function deleteFolder(id: number): Promise<void> {
  if (!inTauri) return;
  return invoke("delete_folder", { id });
}

export async function moveRecipe(recipeId: number, folderId: number | null): Promise<void> {
  if (!inTauri) return;
  return invoke("move_recipe", { recipeId, folderId });
}

export async function confirmDeleteFolder(name: string): Promise<boolean> {
  const text = `Delete the folder "${name}"? Its recipes stay in your library.`;
  if (!inTauri) return window.confirm(text);
  const { ask } = await import("@tauri-apps/plugin-dialog");
  return ask(text, { title: "Delete folder", kind: "warning" });
}

// ---------- Tags ----------

export async function listTags(): Promise<Tag[]> {
  if (!inTauri) return [{ id: 1, name: "Portuguese", recipe_count: 1 }, { id: 2, name: "Soup", recipe_count: 1 }];
  return invoke<Tag[]>("list_tags");
}

export async function renameTag(id: number, name: string): Promise<void> {
  if (!inTauri) return;
  return invoke("rename_tag", { id, name });
}

export async function deleteTag(id: number): Promise<void> {
  if (!inTauri) return;
  return invoke("delete_tag", { id });
}

export async function confirmDeleteTag(name: string): Promise<boolean> {
  const text = `Remove the tag "${name}" from every recipe?`;
  if (!inTauri) return window.confirm(text);
  const { ask } = await import("@tauri-apps/plugin-dialog");
  return ask(text, { title: "Delete tag", kind: "warning" });
}

/** Ids of recipes matching every word, best matches first. */
export async function searchRecipes(query: string): Promise<number[]> {
  if (!inTauri) return query.toLowerCase().split(/\s+/).every((w) => "caldo verde potatoes kale soup".includes(w)) ? [1] : [];
  return invoke<number[]>("search_recipes", { query });
}
