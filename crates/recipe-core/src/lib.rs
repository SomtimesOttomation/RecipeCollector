//! Core of Recipe Collector: the on-disk recipe library.
//!
//! A library is a folder holding `library.db` (SQLite) and an `images/`
//! folder with one sub-folder per recipe.

use rusqlite::Connection;
use serde::Serialize;
use std::path::{Path, PathBuf};

pub const LIBRARY_FOLDER_NAME: &str = "Recipe Collector";
const SCHEMA_VERSION: i64 = 1;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("file error: {0}")]
    Io(#[from] std::io::Error),
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
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
    pub hero_image: Option<String>,
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

    pub fn info(&self) -> Result<LibraryInfo> {
        let recipe_count = self.conn.query_row("SELECT COUNT(*) FROM recipes", [], |r| r.get(0))?;
        let schema_version = self.conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        Ok(LibraryInfo {
            path: self.root.display().to_string(),
            recipe_count,
            schema_version,
        })
    }

    pub fn list_recipes(&self) -> Result<Vec<RecipeSummary>> {
        let mut stmt = self.conn.prepare(
            "SELECT r.id, r.title, r.total_min,
                    (SELECT file_name FROM images i WHERE i.recipe_id = r.id AND i.is_hero = 1 LIMIT 1)
             FROM recipes r ORDER BY r.updated_at DESC",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(RecipeSummary {
                id: r.get(0)?,
                title: r.get(1)?,
                total_min: r.get(2)?,
                hero_image: r.get(3)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<_, _>>()?)
    }

    /// Minimal insert used by tests until the importer lands.
    pub fn create_recipe(&self, title: &str) -> Result<i64> {
        self.conn.execute("INSERT INTO recipes (title) VALUES (?1)", [title])?;
        Ok(self.conn.last_insert_rowid())
    }
}

fn migrate(conn: &Connection) -> Result<()> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version < 1 {
        conn.execute_batch(SCHEMA_V1)?;
        conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    }
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
        let p = std::env::temp_dir().join(format!("lr-test-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn creates_folder_db_and_schema() {
        let root = temp_root("create");
        let lib = Library::open(&root).unwrap();
        assert!(root.join("library.db").exists());
        assert!(root.join("images").is_dir());
        let info = lib.info().unwrap();
        assert_eq!(info.recipe_count, 0);
        assert_eq!(info.schema_version, 1);
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
        assert_eq!(lib.info().unwrap().recipe_count, 1);
    }

    #[test]
    fn handles_space_in_path() {
        let root = temp_root("apos").join(LIBRARY_FOLDER_NAME);
        let lib = Library::open(&root).unwrap();
        assert!(lib.info().unwrap().path.contains("Recipe Collector"));
    }
}
