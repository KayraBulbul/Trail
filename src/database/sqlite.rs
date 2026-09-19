use std::io;

use crate::types::project::{Project, ProjectDraft};
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

    Ok(conn)
}

pub fn insert_project(conn: &Connection, project: &ProjectDraft) -> Result<()> {
    let uuid = Uuid::new_v4();

    conn.execute(
        "INSERT INTO projects (id, name, directory) VALUES (?1, ?2, ?3)",
        (uuid.to_string(), &project.name, &project.directory),
    )?;

    Ok(())
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
