use crate::types::project::Project;
use rusqlite::{Connection, Result};
use uuid::Uuid;

pub fn create_database() -> Result<Connection> {
    let conn = Connection::open("trail.db")?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            directory TEXT NOT NULL,
            created_at TEXT DEFAULT (datetime('now', 'localtime')),
            updated_at TEXT DEFAULT (datetime('now', 'localtime'))
        )",
        (),
    )?;

    Ok(conn)
}

pub fn insert_project(conn: &Connection, project: &Project) -> Result<()> {
    let uuid = Uuid::new_v4();

    conn.execute(
        "INSERT INTO projects (id, name, directory) VALUES (?1, ?2, ?3)",
        (uuid.to_string(), &project.name, &project.directory),
    )?;

    Ok(())
}
