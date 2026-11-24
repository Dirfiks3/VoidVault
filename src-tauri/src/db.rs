use crate::error::{err, AppResult};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const SCHEMA_VERSION: &str = "1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredItem {
    pub id: String,
    pub category: String,
    pub title: String,
    pub service: String,
    pub username: String,
    pub password: String,
    pub totp_secret: String,
    pub note: String,
    pub favorite: bool,
    pub tags: String,
    pub created_at: String,
    pub updated_at: String,
}

pub fn open(path: &Path) -> AppResult<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(err)?;
    }
    let conn = Connection::open(path).map_err(err)?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA foreign_keys=ON;
         PRAGMA synchronous=NORMAL;
         CREATE TABLE IF NOT EXISTS meta (
             key TEXT PRIMARY KEY,
             value TEXT NOT NULL
         );
         CREATE TABLE IF NOT EXISTS items (
             id TEXT PRIMARY KEY,
             category TEXT NOT NULL,
             title TEXT NOT NULL,
             service TEXT NOT NULL DEFAULT '',
             username TEXT NOT NULL DEFAULT '',
             password TEXT NOT NULL DEFAULT '',
             totp_secret TEXT NOT NULL DEFAULT '',
             note TEXT NOT NULL DEFAULT '',
             favorite INTEGER NOT NULL DEFAULT 0,
             tags TEXT NOT NULL DEFAULT '',
             created_at TEXT NOT NULL,
             updated_at TEXT NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_items_favorite_updated
             ON items(favorite DESC, updated_at DESC);",
    )
    .map_err(err)?;
    Ok(conn)
}

pub fn get_meta(conn: &Connection, key: &str) -> AppResult<Option<String>> {
    conn.query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
        row.get(0)
    })
    .optional()
    .map_err(err)
}

pub fn set_meta(conn: &Connection, key: &str, value: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )
    .map_err(err)?;
    Ok(())
}

pub fn query_items(conn: &Connection) -> AppResult<Vec<StoredItem>> {
    let mut stmt = conn
        .prepare(
            "SELECT id, category, title, service, username, password, totp_secret,
                    note, favorite, tags, created_at, updated_at
             FROM items
             ORDER BY favorite DESC, updated_at DESC",
        )
        .map_err(err)?;
    let rows = stmt
        .query_map([], |row| {
            Ok(StoredItem {
                id: row.get(0)?,
                category: row.get(1)?,
                title: row.get(2)?,
                service: row.get(3)?,
                username: row.get(4)?,
                password: row.get(5)?,
                totp_secret: row.get(6)?,
                note: row.get(7)?,
                favorite: row.get::<_, i64>(8)? != 0,
                tags: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
            })
        })
        .map_err(err)?;
    let mut items = Vec::new();
    for row in rows {
        items.push(row.map_err(err)?);
    }
    Ok(items)
}

pub fn query_item(conn: &Connection, id: &str) -> AppResult<Option<StoredItem>> {
    conn.query_row(
        "SELECT id, category, title, service, username, password, totp_secret,
                note, favorite, tags, created_at, updated_at
         FROM items WHERE id = ?1",
        [id],
        |row| {
            Ok(StoredItem {
                id: row.get(0)?,
                category: row.get(1)?,
                title: row.get(2)?,
                service: row.get(3)?,
                username: row.get(4)?,
                password: row.get(5)?,
                totp_secret: row.get(6)?,
                note: row.get(7)?,
                favorite: row.get::<_, i64>(8)? != 0,
                tags: row.get(9)?,
                created_at: row.get(10)?,
                updated_at: row.get(11)?,
            })
        },
    )
    .optional()
    .map_err(err)
}

pub fn save_item(conn: &Connection, item: &StoredItem) -> AppResult<()> {
    conn.execute(
        "INSERT INTO items(
            id, category, title, service, username, password, totp_secret,
            note, favorite, tags, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
         ON CONFLICT(id) DO UPDATE SET
            category = excluded.category,
            title = excluded.title,
            service = excluded.service,
            username = excluded.username,
            password = excluded.password,
            totp_secret = excluded.totp_secret,
            note = excluded.note,
            favorite = excluded.favorite,
            tags = excluded.tags,
            updated_at = excluded.updated_at",
        params![
            item.id,
            item.category,
            item.title,
            item.service,
            item.username,
            item.password,
            item.totp_secret,
            item.note,
            item.favorite as i64,
            item.tags,
            item.created_at,
            item.updated_at,
        ],
    )
    .map_err(err)?;
    Ok(())
}

pub fn delete_item(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM items WHERE id = ?1", [id]).map_err(err)?;
    Ok(())
}

pub fn delete_all_items(conn: &Connection) -> AppResult<()> {
    conn.execute("DELETE FROM items", []).map_err(err)?;
    Ok(())
}
