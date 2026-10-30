use receitas_core::{Library, LibraryInfo, RecipeSummary, LIBRARY_FOLDER_NAME};
use std::sync::Mutex;
use tauri::{Manager, State};

struct AppState(Mutex<Library>);

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[tauri::command]
fn library_info(state: State<AppState>) -> Result<LibraryInfo, String> {
    state.0.lock().map_err(err)?.info().map_err(err)
}

#[tauri::command]
fn list_recipes(state: State<AppState>) -> Result<Vec<RecipeSummary>, String> {
    state.0.lock().map_err(err)?.list_recipes().map_err(err)
}

#[tauri::command]
fn reveal_library(state: State<AppState>) -> Result<(), String> {
    let db = state.0.lock().map_err(err)?.root().join("library.db");
    tauri_plugin_opener::reveal_item_in_dir(db).map_err(err)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Default library location: Documents/Lolo's Receitas
            let docs = app.path().document_dir()?;
            let library = Library::open(docs.join(LIBRARY_FOLDER_NAME))?;
            app.manage(AppState(Mutex::new(library)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![library_info, list_recipes, reveal_library])
        .run(tauri::generate_context!())
        .expect("error while running Lolo's Receitas");
}
