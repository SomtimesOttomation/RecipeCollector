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
    "print_page",
    "check_for_update",
    "install_update",
    "app_version",
    "list_folders",
    "create_folder",
    "rename_folder",
    "delete_folder",
    "move_recipe",
    "list_tags",
    "rename_tag",
    "delete_tag",
    "search_recipes",
];

fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("failed to run tauri build script");
}
