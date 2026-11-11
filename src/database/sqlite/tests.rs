use super::*;
use std::{fs, thread};

#[test]
fn writes_wait_for_another_connection_instead_of_failing() {
    let dir = std::env::temp_dir().join(format!("trail-db-{}", Uuid::new_v4()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("trail.db");
    let tui = open_database(&path).unwrap();
    let cli = open_database(&path).unwrap();

    tui.execute_batch("BEGIN IMMEDIATE").unwrap();
    let writer = thread::spawn(move || {
        thread::sleep(Duration::from_millis(200));
        tui.execute_batch("COMMIT").unwrap();
    });
    let inserted = insert_project(
        &cli,
        &ProjectDraft {
            name: Some("Trail".into()),
            directory: Some("/trail".into()),
        },
    );
    writer.join().unwrap();

    assert!(inserted.is_ok(), "{inserted:?}");
    assert_eq!(get_projects(&cli).unwrap().len(), 1);
    // Windows can't delete a database file that's still open.
    drop(cli);
    fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn migration_keeps_old_updates_and_old_style_inserts_working() {
    let conn = Connection::open_in_memory().unwrap();
    // The updates table as created by v0.7.0 and earlier.
    conn.execute_batch(
        "CREATE TABLE projects (id TEXT PRIMARY KEY, name TEXT NOT NULL, directory TEXT NOT NULL,
             created_at TEXT DEFAULT (datetime('now')), updated_at TEXT DEFAULT (datetime('now')));
         CREATE TABLE updates (id TEXT PRIMARY KEY, project_id TEXT, title TEXT NOT NULL,
             body TEXT NOT NULL, next TEXT NOT NULL,
             created_at TEXT DEFAULT (datetime('now')), updated_at TEXT DEFAULT (datetime('now')),
             FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE);
         INSERT INTO projects (id, name, directory) VALUES ('a', 'Alpha', '/alpha');
         INSERT INTO updates (id, project_id, title, body, next) VALUES ('old', 'a', 'T', 'B', 'N');",
    )
    .unwrap();

    initialize_schema(&conn).unwrap();
    initialize_schema(&conn).unwrap();

    let old = get_update(&conn, "old").unwrap();
    assert_eq!(
        (old.title.as_str(), old.branch, old.commit_sha),
        ("T", None, None)
    );
    let version: i32 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, 1);
    // An older Trail binary opening the migrated file still inserts the way it always did.
    conn.execute(
        "INSERT INTO updates (id, project_id, title, body, next) VALUES ('older', 'a', 'T', 'B', 'N')",
        (),
    )
    .unwrap();
    assert_eq!(get_updates(&conn, "a").unwrap().len(), 2);
    // The plan table is created for databases from before plans existed.
    insert_plan_item(&conn, "a", "Item").unwrap();
    assert_eq!(get_plan_items(&conn, "a").unwrap().len(), 1);
}

#[test]
fn deleting_updates_and_projects_cleans_up_their_plan_items() {
    let conn = Connection::open_in_memory().unwrap();
    initialize_schema(&conn).unwrap();
    let project = |name: &str| ProjectDraft {
        name: Some(name.into()),
        directory: Some(format!("/{name}")),
    };
    let (kept, deleted) = (
        insert_project(&conn, &project("kept")).unwrap(),
        insert_project(&conn, &project("deleted")).unwrap(),
    );
    let update = insert_update(
        &conn,
        &crate::types::update::UpdateDraft {
            title: Some("T".into()),
            project_id: Some(kept.clone()),
            body: Some("B".into()),
            next: Some("N".into()),
            branch: None,
            commit_sha: None,
        },
    )
    .unwrap();
    let item = insert_plan_item(&conn, &kept, "Finished").unwrap();
    close_plan_item(&conn, item, PlanState::Done, Some(&update)).unwrap();
    insert_plan_item(&conn, &deleted, "Gone").unwrap();

    delete_update(&conn, &update).unwrap();
    delete_project(&conn, &deleted).unwrap();

    let item = get_plan_item(&conn, item).unwrap().unwrap();
    assert_eq!((item.state, item.closed_by_update), (PlanState::Done, None));
    let orphans: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM plan_items WHERE project_id = ?1",
            [&deleted],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(orphans, 0);
}
