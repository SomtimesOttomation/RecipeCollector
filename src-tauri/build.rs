// Every app command gets its own permission, so each window is granted only what it needs.
// The in-app browser (which shows outside websites) may call import_page_html and nothing else.
const COMMANDS: &[&str] = &[
    "library_info",
    "list_recipes",
    "reveal_library",
    "import_recipe",
    "open_import_browser",
    "import_page_html",
    "save_debug_page",
    "get_recipe",
    "update_recipe",
    "delete_recipe",
    "add_image_from_file",
    "set_hero",
    "remove_image",
];

fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("failed to run tauri build script");
}
