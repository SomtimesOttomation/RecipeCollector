# Changelog

All notable changes to Recipe Collector. Each entry is labelled Feature, Bug/Fix, Redesign or Housekeeping.

## Unreleased

## 0.3.0

**Feature**
- Folders: create, rename and delete folders, drag recipes onto them, and see your library listed in the sidebar
- Tags: added automatically from the recipe's category, cuisine, diet and keywords, or by hand; browse by tag in the sidebar
- Search across titles, ingredients, steps, notes and tags, with advanced filters for tags, author, site, time and folder
- Units converter: switch any recipe between Auto (as written), Metric and US, including oven temperatures

**Bug/Fix**
- Updater understands release tags like v.0.2.0

## 0.2.0

**Feature**
- App skeleton: sidebar with Library and Settings, light/dark/system themes, native window styling on Windows and macOS
- Recipe library created automatically in Documents/Recipe Collector (database + images folder)
- Automatic Windows (setup .exe) and macOS (.dmg) builds with checksums
- Import recipes from a link (most recipe sites and Pinterest pins) with photos saved locally
- Editable recipe page: ingredients and steps cards, times, servings, notes and photos, saved automatically
- Built-in browser to import from sites that block automatic downloads
- In-app updates: checks GitHub releases and installs new versions
- Save a page for import analysis from the built-in browser
- Print recipes with a clean layout (or save as PDF)
- YouTube video link on recipes, found automatically or added by hand

**Bug/Fix**
- Windows installer installs per user, so no admin (UAC) prompt
- Clearer messages when an import fails

**Housekeeping**
- App named Recipe Collector
