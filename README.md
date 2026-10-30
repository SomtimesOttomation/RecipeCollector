# Recipe Collector

A desktop recipe collector for Windows and macOS. Paste a recipe link and it becomes a clean, fully editable recipe page with an ingredients card, a steps card and the photos saved locally.

Built with [Tauri 2](https://tauri.app) (Rust) and Svelte.

## Status

Early development. Import recipes from most sites (with a built-in browser for the tricky ones), edit everything, print, and update from inside the app. Folders, tags, search and a units converter are next.

## Download

Installers are attached to each [release](../../releases):

- **Windows:** setup `.exe`. Installs for your user only, so no admin prompt. If SmartScreen appears, choose **More info → Run anyway**.
- **macOS:** `.dmg`. The app isn't notarized yet: on first launch, open **System Settings → Privacy & Security** and click **Open Anyway**.

## Your recipes

Recipes are stored in `Documents/Recipe Collector` (a `library.db` file plus an `images` folder), so they're easy to back up.

## Building from source

Needs Node.js 22+ and Rust (plus Visual Studio Build Tools on Windows, or Xcode Command Line Tools on macOS).

```
npm install
npm run tauri dev     # run the app
npm run tauri build   # make installers
```
