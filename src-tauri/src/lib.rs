use recipe_core::{import, update, ImageInfo, Library, LibraryInfo, Recipe, RecipeSummary, LIBRARY_FOLDER_NAME};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

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

/// Saves a parsed recipe and downloads up to four of its photos. A failed photo never fails the import.
fn save_with_photos(state: &AppState, parsed: &recipe_core::parse::ParsedRecipe) -> Result<i64, String> {
    let id = state.with(|l| l.save_parsed(parsed))?;
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
        save_with_photos(&state, &parsed)
    })
    .await
    .map_err(err)?
}

const IMPORT_BAR: &str = include_str!("import_bar.js");
const BROWSER_LABEL: &str = "importer";

/// Opens the page in an in-app browser window with an "Import this page" bar,
/// for sites that block automatic downloads or build the recipe with JavaScript.
/// Must be async: on Windows, creating a window from a synchronous command deadlocks.
#[tauri::command]
async fn open_import_browser(url: String, app: AppHandle) -> Result<(), String> {
    let target: tauri::Url = import::normalize_url(&url).map_err(err)?.parse().map_err(err)?;
    if let Some(existing) = app.get_webview_window(BROWSER_LABEL) {
        existing.navigate(target).map_err(err)?;
        let _ = existing.set_focus();
        return Ok(());
    }
    WebviewWindowBuilder::new(&app, BROWSER_LABEL, WebviewUrl::External(target))
        .title("Recipe Collector — find the recipe, then click Import this page")
        .inner_size(1100.0, 820.0)
        .initialization_script(IMPORT_BAR)
        .build()
        .map_err(err)?;
    Ok(())
}

/// Called by the bar in the browser window with the page as the user sees it.
#[tauri::command]
async fn import_page_html(html: String, url: String, app: AppHandle) -> Result<i64, String> {
    let state = app.state::<AppState>().inner().clone();
    let id = tauri::async_runtime::spawn_blocking(move || {
        let parsed = import::parse_html(&html, &url).map_err(err)?;
        save_with_photos(&state, &parsed)
    })
    .await
    .map_err(err)??;
    let _ = app.emit_to("main", "recipe-imported", id);
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.set_focus();
    }
    if let Some(browser) = app.get_webview_window(BROWSER_LABEL) {
        let _ = browser.close();
    }
    Ok(id)
}

#[tauri::command]
async fn check_for_update(app: AppHandle) -> Result<update::UpdateStatus, String> {
    let current = app.package_info().version.to_string();
    tauri::async_runtime::spawn_blocking(move || update::check(&current)).await.map_err(err)
}

/// Downloads and verifies the installer, starts it, then closes the app so files can be replaced.
#[tauri::command]
async fn install_update(installer_url: String, checksum_url: Option<String>, app: AppHandle) -> Result<(), String> {
    let dir = std::env::temp_dir().join("RecipeCollector-update");
    let path = tauri::async_runtime::spawn_blocking(move || {
        update::download_installer(&installer_url, checksum_url.as_deref(), &dir)
    })
    .await
    .map_err(err)?
    .map_err(err)?;

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new(&path).spawn().map_err(err)?;
        app.exit(0);
    }
    #[cfg(target_os = "macos")]
    {
        // Opens the disk image; the user drags the new app over the old one.
        std::process::Command::new("open").arg(&path).spawn().map_err(err)?;
        let _ = &app;
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        let _ = (&app, &path);
    }
    Ok(())
}

#[tauri::command]
fn app_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
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
            open_import_browser,
            import_page_html,
            get_recipe,
            update_recipe,
            delete_recipe,
            add_image_from_file,
            set_hero,
            remove_image,
            check_for_update,
            install_update,
            app_version
        ])
        .run(tauri::generate_context!())
        .expect("error while running Recipe Collector");
}
