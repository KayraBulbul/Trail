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
    fs::remove_dir_all(&dir).unwrap();
}
