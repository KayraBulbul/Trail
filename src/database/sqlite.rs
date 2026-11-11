use std::{io, path::Path, time::Duration};

use crate::types::{
    plan::{PlanItem, PlanState},
    project::{Project, ProjectDraft},
    update::{Update, UpdateDraft},
};
use rusqlite::{Connection, OptionalExtension, Result, Row};
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

    Ok(open_database(&trail_dir.join("trail.db"))?)
}

/// How long to wait for another Trail process (the TUI or a CLI command) to
/// finish writing before giving up with "database is locked". rusqlite's
/// default is the same today; setting it here keeps us from relying on that.
const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

pub fn open_database(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.busy_timeout(BUSY_TIMEOUT)?;
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

    // v1.1.0: a new table rather than new columns, so older binaries never see it.
    // `id` is an integer so agents can type it: `trail plan done 7`.
    conn.execute(
        "CREATE TABLE IF NOT EXISTS plan_items (
              id INTEGER PRIMARY KEY,
              project_id TEXT NOT NULL,
              text TEXT NOT NULL,
              position INTEGER NOT NULL,
              state TEXT NOT NULL DEFAULT 'open',
              created_at TEXT DEFAULT (datetime('now')),
              closed_at TEXT,
              closed_by_update TEXT,
              FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
        )",
        (),
    )?;

    migrate(conn)
}

/// Bumped by each migration below; stored in SQLite's `user_version` pragma.
const SCHEMA_VERSION: i32 = 1;

/// Brings an existing database up to `SCHEMA_VERSION`. Migrations only add
/// nullable columns, so older versions of Trail can still read and write the file.
fn migrate(conn: &Connection) -> Result<()> {
    let version: i32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version >= SCHEMA_VERSION {
        return Ok(());
    }

    let tx = conn.unchecked_transaction()?;
    if version < 1 {
        // v0.7.1: the branch and commit an update was written on.
        tx.execute_batch(
            "ALTER TABLE updates ADD COLUMN branch TEXT;
             ALTER TABLE updates ADD COLUMN commit_sha TEXT;",
        )?;
    }
    tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    tx.commit()
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
    // Foreign keys are off on Trail's connections, so ON DELETE CASCADE doesn't run.
    let tx = conn.unchecked_transaction()?;
    tx.execute("DELETE FROM plan_items WHERE project_id = ?1", [project_id])?;
    tx.execute("DELETE FROM projects WHERE id = ?1", [project_id])?;
    tx.commit()
}

pub fn insert_update(conn: &Connection, update: &UpdateDraft) -> Result<String> {
    let uuid = Uuid::new_v4().to_string();

    conn.execute(
        "INSERT INTO updates (id, project_id, title, body, next, branch, commit_sha)
              VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        (
            &uuid,
            &update.project_id,
            &update.title,
            &update.body,
            &update.next,
            &update.branch,
            &update.commit_sha,
        ),
    )?;

    Ok(uuid)
}

pub fn get_updates(conn: &Connection, project_id: &str) -> Result<Vec<Update>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, title, body, next, created_at, updated_at, branch, commit_sha
              FROM updates
              WHERE project_id = ?1
              ORDER BY updated_at DESC, rowid DESC",
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
            branch: row.get(7)?,
            commit_sha: row.get(8)?,
        })
    })?;

    update_iter.collect()
}

pub fn get_update(conn: &Connection, update_id: &str) -> Result<Update> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, title, body, next, created_at, updated_at, branch, commit_sha
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
            branch: row.get(7)?,
            commit_sha: row.get(8)?,
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
    // Items the update closed stay closed; they just lose the link to it.
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "UPDATE plan_items SET closed_by_update = NULL WHERE closed_by_update = ?1",
        [update_id],
    )?;
    tx.execute("DELETE FROM updates WHERE id = ?1", [update_id])?;
    tx.commit()
}

const PLAN_ITEM_COLUMNS: &str =
    "id, project_id, text, position, state, created_at, closed_at, closed_by_update";

fn plan_item(row: &Row) -> Result<PlanItem> {
    Ok(PlanItem {
        id: row.get(0)?,
        project_id: row.get(1)?,
        text: row.get(2)?,
        position: row.get(3)?,
        state: PlanState::parse(&row.get::<_, String>(4)?),
        created_at: row.get(5)?,
        closed_at: row.get(6)?,
        closed_by_update: row.get(7)?,
    })
}

/// Adds an open item at the end of the project's plan.
pub fn insert_plan_item(conn: &Connection, project_id: &str, text: &str) -> Result<i64> {
    conn.execute(
        "INSERT INTO plan_items (project_id, text, position)
              VALUES (?1, ?2, (SELECT COALESCE(MAX(position), 0) + 1
                               FROM plan_items WHERE project_id = ?1))",
        (project_id, text),
    )?;

    Ok(conn.last_insert_rowid())
}

/// Every item in the project's plan, open and closed, in plan order.
pub fn get_plan_items(conn: &Connection, project_id: &str) -> Result<Vec<PlanItem>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {PLAN_ITEM_COLUMNS} FROM plan_items
              WHERE project_id = ?1
              ORDER BY position ASC, id ASC"
    ))?;

    stmt.query_map([project_id], plan_item)?.collect()
}

pub fn get_plan_item(conn: &Connection, id: i64) -> Result<Option<PlanItem>> {
    conn.query_row(
        &format!("SELECT {PLAN_ITEM_COLUMNS} FROM plan_items WHERE id = ?1"),
        [id],
        plan_item,
    )
    .optional()
}

/// Marks an item done or dropped now. `update_id` is the update that closed it, if any.
pub fn close_plan_item(
    conn: &Connection,
    id: i64,
    state: PlanState,
    update_id: Option<&str>,
) -> Result<()> {
    let changed = conn.execute(
        "UPDATE plan_items
              SET state = ?1, closed_at = datetime('now'), closed_by_update = ?2
              WHERE id = ?3",
        (state.as_str(), update_id, id),
    )?;

    if changed == 0 {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    Ok(())
}

pub fn reopen_plan_item(conn: &Connection, id: i64) -> Result<()> {
    let changed = conn.execute(
        "UPDATE plan_items
              SET state = 'open', closed_at = NULL, closed_by_update = NULL
              WHERE id = ?1",
        [id],
    )?;

    if changed == 0 {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    Ok(())
}

pub fn edit_plan_item(conn: &Connection, id: i64, text: &str) -> Result<()> {
    let changed = conn.execute("UPDATE plan_items SET text = ?1 WHERE id = ?2", (text, id))?;

    if changed == 0 {
        return Err(rusqlite::Error::QueryReturnedNoRows);
    }
    Ok(())
}

/// Swaps two items' places in the plan.
pub fn swap_plan_items(conn: &Connection, first: &PlanItem, second: &PlanItem) -> Result<()> {
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "UPDATE plan_items SET position = ?1 WHERE id = ?2",
        (second.position, first.id),
    )?;
    tx.execute(
        "UPDATE plan_items SET position = ?1 WHERE id = ?2",
        (first.position, second.id),
    )?;
    tx.commit()
}

pub fn delete_plan_item(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM plan_items WHERE id = ?1", [id])?;

    Ok(())
}

#[cfg(test)]
mod tests;
