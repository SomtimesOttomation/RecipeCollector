use recipe_core::{import, ImageInfo, Library, LibraryInfo, Recipe, RecipeSummary, LIBRARY_FOLDER_NAME};
use std::sync::{Arc, Mutex};
use tauri::{Manager, State};

/// Shared library handle. Network work happens outside the lock so the UI
/// stays responsive while a recipe downloads.
#[derive(Clone)]
struct AppState(Arc<Mutex<Library>>);

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

impl AppState {
    fn with<T>(&self, f: impl FnOnce(&mut Library) -> recipe_core::Result<T>) -> Result<T, String> {
        let mut lib = self.0.lock().map_err(err)?;
        f(&mut lib).map_err(err)
    }
}

#[tauri::command]
fn library_info(state: State<AppState>) -> Result<LibraryInfo, String> {
    state.with(|l| l.info())
}

#[tauri::command]
fn list_recipes(state: State<AppState>) -> Result<Vec<RecipeSummary>, String> {
    state.with(|l| l.list_recipes())
}

#[tauri::command]
fn reveal_library(state: State<AppState>) -> Result<(), String> {
    let db = state.with(|l| Ok(l.root().join("library.db")))?;
    tauri_plugin_opener::reveal_item_in_dir(db).map_err(err)
}

/// Imports a recipe from a link: fetch + parse, save, then download photos.
#[tauri::command]
async fn import_recipe(url: String, state: State<'_, AppState>) -> Result<i64, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let parsed = import::fetch_recipe(&url).map_err(err)?;
        let id = state.with(|l| l.save_parsed(&parsed))?;
        // Cover photo first, then up to three more. A failed photo never fails the import.
        let mut saved = 0;
        for img_url in parsed.images.iter().take(6) {
            if saved >= 4 {
                break;
            }
            if let Ok(bytes) = import::download_image(img_url) {
                if state.with(|l| l.add_image(id, &bytes, Some(img_url), false)).is_ok() {
                    saved += 1;
                }
            }
        }
        Ok(id)
    })
    .await
    .map_err(err)?
}

#[tauri::command]
fn get_recipe(id: i64, state: State<AppState>) -> Result<Recipe, String> {
    state.with(|l| l.get_recipe(id))
}

#[tauri::command]
fn update_recipe(recipe: Recipe, state: State<AppState>) -> Result<(), String> {
    state.with(|l| l.update_recipe(&recipe))
}

#[tauri::command]
fn delete_recipe(id: i64, state: State<AppState>) -> Result<(), String> {
    state.with(|l| l.delete_recipe(id))
}

#[tauri::command]
fn add_image_from_file(recipe_id: i64, path: String, make_hero: bool, state: State<AppState>) -> Result<ImageInfo, String> {
    let bytes = std::fs::read(&path).map_err(err)?;
    state.with(|l| l.add_image(recipe_id, &bytes, None, make_hero))
}

#[tauri::command]
fn set_hero(image_id: i64, state: State<AppState>) -> Result<(), String> {
    state.with(|l| l.set_hero(image_id))
}

#[tauri::command]
fn remove_image(image_id: i64, state: State<AppState>) -> Result<(), String> {
    state.with(|l| l.remove_image(image_id))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // Default library location: Documents/Recipe Collector
            let docs = app.path().document_dir()?;
            let library = Library::open(docs.join(LIBRARY_FOLDER_NAME))?;
            app.manage(AppState(Arc::new(Mutex::new(library))));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            library_info,
            list_recipes,
            reveal_library,
            import_recipe,
            get_recipe,
            update_recipe,
            delete_recipe,
            add_image_from_file,
            set_hero,
            remove_image
        ])
        .run(tauri::generate_context!())
        .expect("error while running Recipe Collector");
}
