use std::io;

use crate::types::{
    project::{Project, ProjectDraft},
    update::{Update, UpdateDraft},
};
use rusqlite::{Connection, Result};
use uuid::Uuid;

pub fn create_database() -> Result<Connection, Box<dyn std::error::Error>> {
    let data_dir = dirs::data_local_dir().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "Unable to find local data directory.",
        )
    })?;

    let trail_dir = data_dir.join("trail");
    std::fs::create_dir_all(&trail_dir)?;

    let conn = Connection::open(trail_dir.join("trail.db"))?;
    initialize_schema(&conn)?;

    Ok(conn)
}

pub fn initialize_schema(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            directory TEXT NOT NULL,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now'))
        )",
        (),
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS updates (
              id TEXT PRIMARY KEY,
              project_id TEXT,
              title TEXT NOT NULL,
              body TEXT NOT NULL,
              next TEXT NOT NULL,
              created_at TEXT DEFAULT (datetime('now')),
              updated_at TEXT DEFAULT (datetime('now')),
              FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
        )",
        (),
    )?;

    Ok(())
}

pub fn insert_project(conn: &Connection, project: &ProjectDraft) -> Result<String> {
    let uuid = Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO projects (id, name, directory) VALUES (?1, ?2, ?3)",
        (&uuid, &project.name, &project.directory),
    )?;

    Ok(uuid)
}

pub fn get_projects(conn: &Connection) -> Result<Vec<Project>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, directory, created_at, updated_at
              FROM projects
              ORDER BY updated_at DESC, id ASC",
    )?;

    let project_iter = stmt.query_map([], |row| {
        Ok(Project {
            id: row.get(0)?,
            name: row.get(1)?,
            directory: row.get(2)?,
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
        })
    })?;

    project_iter.collect()
}

pub fn delete_project(conn: &Connection, project_id: &str) -> Result<()> {
    conn.execute("DELETE FROM projects WHERE id = ?1", [project_id])?;

    Ok(())
}

pub fn insert_update(conn: &Connection, update: &UpdateDraft) -> Result<String> {
    let uuid = Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO updates (id, project_id, title, body, next) VALUES (?1, ?2, ?3, ?4, ?5)",
        (
            &uuid,
            &update.project_id,
            &update.title,
            &update.body,
            &update.next,
        ),
    )?;

    Ok(uuid)
}

pub fn get_updates(conn: &Connection, project_id: &str) -> Result<Vec<Update>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, title, body, next, created_at, updated_at
              FROM updates
              WHERE project_id = ?1
              ORDER BY updated_at DESC, id ASC",
    )?;

    let update_iter = stmt.query_map([project_id], |row| {
        Ok(Update {
            id: row.get(0)?,
            project_id: row.get(1)?,
            title: row.get(2)?,
            body: row.get(3)?,
            next: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
        })
    })?;

    update_iter.collect()
}

pub fn get_update(conn: &Connection, update_id: &str) -> Result<Update> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, title, body, next, created_at, updated_at
         FROM updates WHERE id = ?1",
    )?;
    let update = stmt.query_row([update_id], |row| {
        Ok(Update {
            id: row.get(0)?,
            project_id: row.get(1)?,
            title: row.get(2)?,
            body: row.get(3)?,
            next: row.get(4)?,
            created_at: row.get(5)?,
            updated_at: row.get(6)?,
        })
    })?;

    Ok(update)
}

pub fn edit_update(conn: &Connection, update: &Update) -> Result<()> {
    let changed = conn.execute(
        "UPDATE updates
              SET title = ?1, body = ?2, next = ?3, updated_at = datetime('now')
              WHERE id = ?4",
        (&update.title, &update.body, &update.next, &update.id),
    )?;

    if changed == 0 {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    Ok(())
}

pub fn delete_update(conn: &Connection, update_id: &str) -> Result<()> {
    conn.execute("DELETE FROM updates WHERE id = ?1", [update_id])?;

    Ok(())
}
