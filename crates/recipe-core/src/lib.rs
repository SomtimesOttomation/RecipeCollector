//! Core of Recipe Collector: the on-disk recipe library and recipe import.
//!
//! A library is a folder holding `library.db` (SQLite) and an `images/`
//! folder with one sub-folder per recipe.

pub mod import;
pub mod parse;
pub mod update;

use parse::{ParsedRecipe, Section};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const LIBRARY_FOLDER_NAME: &str = "Recipe Collector";
const SCHEMA_VERSION: i64 = 3;
const MAX_IMAGE_EDGE: u32 = 2000;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("file error: {0}")]
    Io(#[from] std::io::Error),
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("couldn't reach the site: {0}")]
    Network(String),
    #[error("this site blocks automatic downloads")]
    Blocked,
    #[error("that doesn't look like a web address")]
    BadUrl,
    #[error("couldn't find a recipe on that page")]
    NotARecipe,
    #[error("that pin doesn't link to a recipe site")]
    PinWithoutLink,
    #[error("couldn't read that image: {0}")]
    Image(String),
    #[error("recipe not found")]
    NotFound,
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Serialize)]
pub struct LibraryInfo {
    pub path: String,
    pub recipe_count: i64,
    pub schema_version: i64,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct RecipeSummary {
    pub id: i64,
    pub title: String,
    pub total_min: Option<i64>,
    pub site_name: Option<String>,
    /// Absolute path of the cover photo, if any.
    pub hero_image: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImageInfo {
    pub id: i64,
    pub path: String,
    pub is_hero: bool,
}

/// A full recipe as shown and edited on the recipe page.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Recipe {
    pub id: i64,
    pub title: String,
    pub description: Option<String>,
    pub source_url: Option<String>,
    pub site_name: Option<String>,
    pub author: Option<String>,
    pub servings: Option<String>,
    pub prep_min: Option<i64>,
    pub cook_min: Option<i64>,
    pub total_min: Option<i64>,
    pub notes: Option<String>,
    #[serde(default)]
    pub video_url: Option<String>,
    pub ingredients: Vec<Section>,
    pub steps: Vec<Section>,
    #[serde(default)]
    pub images: Vec<ImageInfo>,
}

pub struct Library {
    root: PathBuf,
    conn: Connection,
}

impl Library {
    /// Opens the library in `root`, creating the folder, database and schema if needed.
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        std::fs::create_dir_all(root.join("images"))?;
        let conn = Connection::open(root.join("library.db"))?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        migrate(&conn)?;
        Ok(Self { root, conn })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn images_dir(&self, recipe_id: i64) -> PathBuf {
        self.root.join("images").join(recipe_id.to_string())
    }

    pub fn info(&self) -> Result<LibraryInfo> {
        let recipe_count = self.conn.query_row("SELECT COUNT(*) FROM recipes", [], |r| r.get(0))?;
        let schema_version = self.conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        Ok(LibraryInfo { path: self.root.display().to_string(), recipe_count, schema_version })
    }

    pub fn list_recipes(&self) -> Result<Vec<RecipeSummary>> {
        let mut stmt = self.conn.prepare(
            "SELECT r.id, r.title, r.total_min, r.site_name,
                    (SELECT file_name FROM images i WHERE i.recipe_id = r.id
                     ORDER BY i.is_hero DESC, i.position LIMIT 1)
             FROM recipes r ORDER BY r.updated_at DESC, r.id DESC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((r.get::<_, i64>(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get::<_, Option<String>>(4)?))
        })?;
        let mut out = vec![];
        for row in rows {
            let (id, title, total_min, site_name, file) = row?;
            out.push(RecipeSummary {
                id,
                title,
                total_min,
                site_name,
                hero_image: file.map(|f| self.images_dir(id).join(f).display().to_string()),
            });
        }
        Ok(out)
    }

    /// Minimal insert, used by tests.
    pub fn create_recipe(&self, title: &str) -> Result<i64> {
        self.conn.execute("INSERT INTO recipes (title) VALUES (?1)", [title])?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Saves an imported recipe (without photos) and returns its id.
    pub fn save_parsed(&mut self, p: &ParsedRecipe) -> Result<i64> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO recipes (title, description, source_url, site_name, author, servings, prep_min, cook_min, total_min, video_url)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![p.title, p.description, p.source_url, p.site_name, p.author, p.servings, p.prep_min, p.cook_min, p.total_min, p.video_url],
        )?;
        let id = tx.last_insert_rowid();
        write_sections(&tx, id, &p.ingredients, &p.steps)?;
        tx.execute(
            "INSERT INTO raw_imports (recipe_id, json_ld) VALUES (?1, ?2)",
            params![id, p.json_ld],
        )?;
        tx.commit()?;
        Ok(id)
    }

    pub fn get_recipe(&self, id: i64) -> Result<Recipe> {
        let mut r = self
            .conn
            .query_row(
                "SELECT id, title, description, source_url, site_name, author, servings, prep_min, cook_min, total_min, notes, video_url
                 FROM recipes WHERE id = ?1",
                [id],
                |r| {
                    Ok(Recipe {
                        id: r.get(0)?,
                        title: r.get(1)?,
                        description: r.get(2)?,
                        source_url: r.get(3)?,
                        site_name: r.get(4)?,
                        author: r.get(5)?,
                        servings: r.get(6)?,
                        prep_min: r.get(7)?,
                        cook_min: r.get(8)?,
                        total_min: r.get(9)?,
                        notes: r.get(10)?,
                        video_url: r.get(11)?,
                        ..Default::default()
                    })
                },
            )
            .optional()?
            .ok_or(Error::NotFound)?;

        // Ingredients grouped by section.
        let mut sec_stmt = self.conn.prepare("SELECT id, title FROM ingredient_sections WHERE recipe_id = ?1 ORDER BY position")?;
        let secs: Vec<(i64, Option<String>)> =
            sec_stmt.query_map([id], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<std::result::Result<_, _>>()?;
        let mut item_stmt = self.conn.prepare("SELECT raw_text FROM ingredients WHERE section_id = ?1 ORDER BY position")?;
        for (sid, title) in secs {
            let items = item_stmt.query_map([sid], |r| r.get(0))?.collect::<std::result::Result<Vec<String>, _>>()?;
            r.ingredients.push(Section { title, items });
        }

        // Steps, grouped by their section title.
        let mut step_stmt = self.conn.prepare("SELECT section_title, text FROM steps WHERE recipe_id = ?1 ORDER BY position")?;
        let rows = step_stmt.query_map([id], |r| Ok((r.get::<_, Option<String>>(0)?, r.get::<_, String>(1)?)))?;
        let mut last_title: Option<Option<String>> = None;
        for row in rows {
            let (title, text) = row?;
            if last_title.as_ref() != Some(&title) {
                r.steps.push(Section { title: title.clone(), items: vec![] });
                last_title = Some(title);
            }
            r.steps.last_mut().unwrap().items.push(text);
        }

        r.images = self.images_of(id)?;
        Ok(r)
    }

    fn images_of(&self, id: i64) -> Result<Vec<ImageInfo>> {
        let mut stmt = self.conn.prepare("SELECT id, file_name, is_hero FROM images WHERE recipe_id = ?1 ORDER BY is_hero DESC, position, id")?;
        let dir = self.images_dir(id);
        let rows = stmt.query_map([id], |r| {
            Ok(ImageInfo { id: r.get(0)?, path: dir.join(r.get::<_, String>(1)?).display().to_string(), is_hero: r.get::<_, i64>(2)? != 0 })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// Saves every edited field of a recipe (photos are managed separately).
    pub fn update_recipe(&mut self, r: &Recipe) -> Result<()> {
        let tx = self.conn.transaction()?;
        let changed = tx.execute(
            "UPDATE recipes SET title=?2, description=?3, source_url=?4, site_name=?5, author=?6, servings=?7,
                    prep_min=?8, cook_min=?9, total_min=?10, notes=?11, video_url=?12, updated_at=datetime('now')
             WHERE id=?1",
            params![r.id, r.title, r.description, r.source_url, r.site_name, r.author, r.servings, r.prep_min, r.cook_min, r.total_min, r.notes, r.video_url],
        )?;
        if changed == 0 {
            return Err(Error::NotFound);
        }
        tx.execute("DELETE FROM ingredient_sections WHERE recipe_id=?1", [r.id])?;
        tx.execute("DELETE FROM steps WHERE recipe_id=?1", [r.id])?;
        write_sections(&tx, r.id, &r.ingredients, &r.steps)?;
        tx.commit()?;
        Ok(())
    }

    pub fn delete_recipe(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM recipes WHERE id=?1", [id])?;
        let dir = self.images_dir(id);
        if dir.exists() {
            std::fs::remove_dir_all(dir)?;
        }
        Ok(())
    }

    /// Stores a photo for a recipe: resized to at most 2000 px and saved as JPEG.
    /// The first photo of a recipe becomes its cover automatically.
    pub fn add_image(&self, recipe_id: i64, bytes: &[u8], source_url: Option<&str>, make_hero: bool) -> Result<ImageInfo> {
        let img = image::load_from_memory(bytes).map_err(|e| Error::Image(e.to_string()))?;
        let img = if img.width().max(img.height()) > MAX_IMAGE_EDGE {
            img.resize(MAX_IMAGE_EDGE, MAX_IMAGE_EDGE, image::imageops::FilterType::Lanczos3)
        } else {
            img
        };
        let dir = self.images_dir(recipe_id);
        std::fs::create_dir_all(&dir)?;
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        let file_name = format!("{stamp}.jpg");
        let path = dir.join(&file_name);
        let rgb = img.to_rgb8();
        let mut out = std::io::BufWriter::new(std::fs::File::create(&path)?);
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 88)
            .encode_image(&rgb)
            .map_err(|e| Error::Image(e.to_string()))?;

        let count: i64 = self.conn.query_row("SELECT COUNT(*) FROM images WHERE recipe_id=?1", [recipe_id], |r| r.get(0))?;
        let hero = make_hero || count == 0;
        if hero {
            self.conn.execute("UPDATE images SET is_hero=0 WHERE recipe_id=?1", [recipe_id])?;
        }
        self.conn.execute(
            "INSERT INTO images (recipe_id, file_name, source_url, is_hero, position, width, height) VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![recipe_id, file_name, source_url, hero as i64, count, rgb.width(), rgb.height()],
        )?;
        self.touch(recipe_id)?;
        Ok(ImageInfo { id: self.conn.last_insert_rowid(), path: path.display().to_string(), is_hero: hero })
    }

    pub fn set_hero(&self, image_id: i64) -> Result<()> {
        let recipe_id: i64 = self
            .conn
            .query_row("SELECT recipe_id FROM images WHERE id=?1", [image_id], |r| r.get(0))
            .optional()?
            .ok_or(Error::NotFound)?;
        self.conn.execute("UPDATE images SET is_hero = (id = ?2) WHERE recipe_id=?1", params![recipe_id, image_id])?;
        self.touch(recipe_id)
    }

    pub fn remove_image(&self, image_id: i64) -> Result<()> {
        let row: Option<(i64, String, i64)> = self
            .conn
            .query_row("SELECT recipe_id, file_name, is_hero FROM images WHERE id=?1", [image_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .optional()?;
        let Some((recipe_id, file, was_hero)) = row else { return Err(Error::NotFound) };
        self.conn.execute("DELETE FROM images WHERE id=?1", [image_id])?;
        let _ = std::fs::remove_file(self.images_dir(recipe_id).join(file));
        if was_hero != 0 {
            // Promote the next photo, if any.
            self.conn.execute(
                "UPDATE images SET is_hero=1 WHERE id = (SELECT id FROM images WHERE recipe_id=?1 ORDER BY position, id LIMIT 1)",
                [recipe_id],
            )?;
        }
        self.touch(recipe_id)
    }

    /// Saves a page and its import report into the library's `debug` folder.
    /// Returns the path of the report.
    pub fn save_debug_page(&self, html: &str, url: &str) -> Result<PathBuf> {
        let dir = self.root.join("debug");
        std::fs::create_dir_all(&dir)?;
        let host: String = url::Url::parse(url)
            .ok()
            .and_then(|u| u.host_str().map(|h| h.trim_start_matches("www.").to_string()))
            .unwrap_or_else(|| "page".into())
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '-' { c } else { '_' })
            .collect();
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let base = format!("{stamp}-{host}");
        std::fs::write(dir.join(format!("{base}.html")), html)?;
        let report_path = dir.join(format!("{base}-report.txt"));
        std::fs::write(&report_path, parse::diagnose(html, url))?;
        Ok(report_path)
    }

    fn touch(&self, recipe_id: i64) -> Result<()> {
        self.conn.execute("UPDATE recipes SET updated_at=datetime('now') WHERE id=?1", [recipe_id])?;
        Ok(())
    }
}

fn write_sections(tx: &rusqlite::Transaction, id: i64, ingredients: &[Section], steps: &[Section]) -> Result<()> {
    for (si, sec) in ingredients.iter().enumerate() {
        tx.execute(
            "INSERT INTO ingredient_sections (recipe_id, title, position) VALUES (?1, ?2, ?3)",
            params![id, sec.title.as_deref().filter(|t| !t.trim().is_empty()), si as i64],
        )?;
        let sid = tx.last_insert_rowid();
        for (ii, item) in sec.items.iter().enumerate() {
            tx.execute(
                "INSERT INTO ingredients (section_id, position, raw_text) VALUES (?1, ?2, ?3)",
                params![sid, ii as i64, item],
            )?;
        }
    }
    let mut pos = 0i64;
    for sec in steps {
        let title = sec.title.as_deref().filter(|t| !t.trim().is_empty());
        for text in &sec.items {
            tx.execute(
                "INSERT INTO steps (recipe_id, position, text, section_title) VALUES (?1, ?2, ?3, ?4)",
                params![id, pos, text, title],
            )?;
            pos += 1;
        }
    }
    Ok(())
}

fn migrate(conn: &Connection) -> Result<()> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version < 1 {
        conn.execute_batch(SCHEMA_V1)?;
    }
    if version < 2 {
        conn.execute_batch("ALTER TABLE steps ADD COLUMN section_title TEXT;")?;
    }
    if version < 3 {
        conn.execute_batch("ALTER TABLE recipes ADD COLUMN video_url TEXT;")?;
    }
    conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    Ok(())
}

const SCHEMA_V1: &str = r#"
BEGIN;
CREATE TABLE recipes (
    id           INTEGER PRIMARY KEY,
    title        TEXT NOT NULL,
    description  TEXT,
    source_url   TEXT,
    site_name    TEXT,
    author       TEXT,
    servings     TEXT,
    prep_min     INTEGER,
    cook_min     INTEGER,
    total_min    INTEGER,
    notes        TEXT,
    rating       INTEGER,
    created_at   TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at   TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE ingredient_sections (
    id         INTEGER PRIMARY KEY,
    recipe_id  INTEGER NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    title      TEXT,
    position   INTEGER NOT NULL
);
CREATE TABLE ingredients (
    id          INTEGER PRIMARY KEY,
    section_id  INTEGER NOT NULL REFERENCES ingredient_sections(id) ON DELETE CASCADE,
    position    INTEGER NOT NULL,
    raw_text    TEXT NOT NULL,
    quantity    REAL,
    unit        TEXT,
    item        TEXT,
    note        TEXT
);
CREATE TABLE images (
    id          INTEGER PRIMARY KEY,
    recipe_id   INTEGER NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    file_name   TEXT NOT NULL,
    source_url  TEXT,
    is_hero     INTEGER NOT NULL DEFAULT 0,
    position    INTEGER NOT NULL DEFAULT 0,
    width       INTEGER,
    height      INTEGER
);
CREATE TABLE steps (
    id         INTEGER PRIMARY KEY,
    recipe_id  INTEGER NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    position   INTEGER NOT NULL,
    text       TEXT NOT NULL,
    image_id   INTEGER REFERENCES images(id) ON DELETE SET NULL,
    timer_sec  INTEGER
);
CREATE TABLE tags (
    id    INTEGER PRIMARY KEY,
    name  TEXT NOT NULL UNIQUE COLLATE NOCASE
);
CREATE TABLE recipe_tags (
    recipe_id  INTEGER NOT NULL REFERENCES recipes(id) ON DELETE CASCADE,
    tag_id     INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (recipe_id, tag_id)
);
CREATE TABLE raw_imports (
    recipe_id   INTEGER PRIMARY KEY REFERENCES recipes(id) ON DELETE CASCADE,
    fetched_at  TEXT NOT NULL DEFAULT (datetime('now')),
    json_ld     TEXT,
    html_hash   TEXT
);
COMMIT;
"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("rc-test-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    fn sample() -> ParsedRecipe {
        ParsedRecipe {
            title: "Caldo verde".into(),
            source_url: Some("https://example.com/caldo".into()),
            servings: Some("4".into()),
            total_min: Some(45),
            ingredients: vec![
                Section { title: None, items: vec!["1 lb potatoes".into(), "kale".into()] },
                Section { title: Some("To serve".into()), items: vec!["olive oil".into()] },
            ],
            steps: vec![
                Section { title: None, items: vec!["Boil.".into(), "Blend.".into()] },
                Section { title: Some("Finish".into()), items: vec!["Drizzle.".into()] },
            ],
            ..Default::default()
        }
    }

    fn png_bytes(w: u32, h: u32) -> Vec<u8> {
        let img = image::RgbImage::from_pixel(w, h, image::Rgb([200, 80, 50]));
        let mut out = std::io::Cursor::new(vec![]);
        image::DynamicImage::ImageRgb8(img).write_to(&mut out, image::ImageFormat::Png).unwrap();
        out.into_inner()
    }

    #[test]
    fn creates_folder_db_and_schema() {
        let root = temp_root("create");
        let lib = Library::open(&root).unwrap();
        assert!(root.join("library.db").exists());
        assert!(root.join("images").is_dir());
        let info = lib.info().unwrap();
        assert_eq!(info.recipe_count, 0);
        assert_eq!(info.schema_version, 3);
    }

    #[test]
    fn reopening_keeps_data() {
        let root = temp_root("reopen");
        {
            let lib = Library::open(&root).unwrap();
            lib.create_recipe("Caldo verde").unwrap();
        }
        let lib = Library::open(&root).unwrap();
        let list = lib.list_recipes().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].title, "Caldo verde");
    }

    #[test]
    fn upgrades_v1_library() {
        let root = temp_root("upgrade");
        std::fs::create_dir_all(&root).unwrap();
        {
            let conn = Connection::open(root.join("library.db")).unwrap();
            conn.execute_batch(SCHEMA_V1).unwrap();
            conn.pragma_update(None, "user_version", 1).unwrap();
            conn.execute("INSERT INTO recipes (title) VALUES ('Old')", []).unwrap();
        }
        let mut lib = Library::open(&root).unwrap();
        assert_eq!(lib.info().unwrap().schema_version, 3);
        let id = lib.save_parsed(&sample()).unwrap();
        assert_eq!(lib.get_recipe(id).unwrap().steps.len(), 2);
    }

    #[test]
    fn save_and_load_round_trip() {
        let mut lib = Library::open(temp_root("roundtrip")).unwrap();
        let id = lib.save_parsed(&sample()).unwrap();
        let r = lib.get_recipe(id).unwrap();
        assert_eq!(r.title, "Caldo verde");
        assert_eq!(r.total_min, Some(45));
        assert_eq!(r.ingredients, sample().ingredients);
        assert_eq!(r.steps, sample().steps);
    }

    #[test]
    fn edits_are_saved() {
        let mut lib = Library::open(temp_root("edit")).unwrap();
        let id = lib.save_parsed(&sample()).unwrap();
        let mut r = lib.get_recipe(id).unwrap();
        r.title = "Caldo verde da avó".into();
        r.notes = Some("Use good chouriço".into());
        r.video_url = Some("https://www.youtube.com/watch?v=abcdefghijk".into());
        r.ingredients[0].items.push("2 cloves garlic".into());
        r.steps = vec![Section { title: None, items: vec!["Only one step now.".into()] }];
        lib.update_recipe(&r).unwrap();
        let again = lib.get_recipe(id).unwrap();
        assert_eq!(again.title, "Caldo verde da avó");
        assert_eq!(again.notes.as_deref(), Some("Use good chouriço"));
        assert_eq!(again.video_url, r.video_url);
        assert_eq!(again.ingredients[0].items.len(), 3);
        assert_eq!(again.steps, r.steps);
    }

    #[test]
    fn photos_resize_hero_and_delete() {
        let mut lib = Library::open(temp_root("photos")).unwrap();
        let id = lib.save_parsed(&sample()).unwrap();
        let a = lib.add_image(id, &png_bytes(3000, 1500), None, false).unwrap();
        assert!(a.is_hero);
        let saved = image::open(&a.path).unwrap();
        assert_eq!((saved.width(), saved.height()), (2000, 1000));
        let b = lib.add_image(id, &png_bytes(100, 100), None, false).unwrap();
        assert!(!b.is_hero);
        lib.set_hero(b.id).unwrap();
        let imgs = lib.get_recipe(id).unwrap().images;
        assert!(imgs[0].is_hero && imgs[0].id == b.id);
        assert_eq!(lib.list_recipes().unwrap()[0].hero_image.as_deref(), Some(b.path.as_str()));
        lib.remove_image(b.id).unwrap();
        let imgs = lib.get_recipe(id).unwrap().images;
        assert_eq!(imgs.len(), 1);
        assert!(imgs[0].is_hero);
        assert!(lib.add_image(id, b"not an image", None, false).is_err());
        lib.delete_recipe(id).unwrap();
        assert!(matches!(lib.get_recipe(id), Err(Error::NotFound)));
        assert!(!Path::new(&a.path).exists());
    }

    #[test]
    fn saves_debug_page() {
        let lib = Library::open(temp_root("debug")).unwrap();
        let report = lib.save_debug_page("<html><title>Hi</title></html>", "https://www.example.com/a?b=c").unwrap();
        assert!(report.exists());
        assert!(report.file_name().unwrap().to_string_lossy().ends_with("-example.com-report.txt"));
        let html = report.with_file_name(report.file_name().unwrap().to_string_lossy().replace("-report.txt", ".html"));
        assert!(html.exists());
    }

    #[test]
    fn handles_space_in_path() {
        let root = temp_root("space").join(LIBRARY_FOLDER_NAME);
        let lib = Library::open(&root).unwrap();
        assert!(lib.info().unwrap().path.contains("Recipe Collector"));
    }
}
