// Thin wrapper over the Rust commands. Falls back to sample data when the UI
// runs in a plain browser (npm run dev) so the layout can be worked on anywhere.
import { invoke } from "@tauri-apps/api/core";

export interface LibraryInfo {
  path: string;
  recipe_count: number;
  schema_version: number;
}

export interface RecipeSummary {
  id: number;
  title: string;
  total_min: number | null;
  hero_image: string | null;
}

const inTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

export async function libraryInfo(): Promise<LibraryInfo> {
  if (!inTauri) return { path: "~/Documents/Lolo's Receitas", recipe_count: 0, schema_version: 1 };
  return invoke<LibraryInfo>("library_info");
}

export async function listRecipes(): Promise<RecipeSummary[]> {
  if (!inTauri) return [];
  return invoke<RecipeSummary[]>("list_recipes");
}

export async function revealLibrary(): Promise<void> {
  if (!inTauri) return;
  return invoke("reveal_library");
}
