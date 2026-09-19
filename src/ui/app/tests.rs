use super::*;
use crossterm::event::{KeyEvent, KeyModifiers};

fn setup() -> (App, input::Input, Connection) {
    (
        App {
            show_project_input: true,
            err: None,
            exit: false,
        },
        input::Input {
            input_mode: InputMode::Editing,
            ..input::Input::new()
        },
        Connection::open_in_memory().unwrap(),
    )
}

fn press(app: &mut App, input: &mut input::Input, conn: &Connection, key: KeyCode) {
    app.handle_key_event(KeyEvent::new(key, KeyModifiers::NONE), input, conn)
        .unwrap();
}

fn submit(app: &mut App, input: &mut input::Input, conn: &Connection, text: &str) {
    for ch in text.chars() {
        press(app, input, conn, KeyCode::Char(ch));
    }
    press(app, input, conn, KeyCode::Enter);
}

#[test]
fn blank_directory_stays_on_directory_step() {
    for name in ["", "Trail"] {
        for directory in ["", "   "] {
            let (mut app, mut input, conn) = setup();
            submit(&mut app, &mut input, &conn, name);
            submit(&mut app, &mut input, &conn, directory);

            assert!(input.project_step == ProjectStep::Directory);
            assert!(input.project.directory.is_none());
            assert!(app.err.is_some());
            assert!(app.show_project_input);
        }
    }
}

#[test]
fn directory_supplies_an_automatic_name() {
    let (mut app, mut input, conn) = setup();
    submit(&mut app, &mut input, &conn, "");
    submit(&mut app, &mut input, &conn, "/tmp/Trail");

    assert_eq!(input.project.name.as_deref(), Some("Trail"));
    assert_eq!(input.project.directory.as_deref(), Some("/tmp/Trail"));
    assert!(input.project_step == ProjectStep::Confirm);
    assert!(input.input.is_empty());
    assert_eq!(input.character_index, 0);
    assert!(app.err.is_none());
}

#[test]
fn root_without_a_name_requests_a_name() {
    let (mut app, mut input, conn) = setup();
    submit(&mut app, &mut input, &conn, "");
    submit(&mut app, &mut input, &conn, "/");

    assert!(input.project_step == ProjectStep::Name);
    assert!(input.project.name.is_none());
    assert_eq!(input.project.directory.as_deref(), Some("/"));
    assert!(app.err.is_some());
    assert!(app.show_project_input);
}

#[test]
fn replacement_name_must_be_nonblank() {
    let (mut app, mut input, conn) = setup();
    submit(&mut app, &mut input, &conn, "");
    submit(&mut app, &mut input, &conn, "/");
    press(&mut app, &mut input, &conn, KeyCode::Backspace);

    for blank in ["", "   "] {
        submit(&mut app, &mut input, &conn, blank);
        assert!(input.project_step == ProjectStep::Name);
        assert!(input.project.name.is_none());
        assert_eq!(input.project.directory.as_deref(), Some("/"));
        assert!(app.err.is_some());
    }

    submit(&mut app, &mut input, &conn, "Root");
    assert!(input.project_step == ProjectStep::Confirm);
    assert_eq!(input.project.name.as_deref(), Some("Root"));
    assert_eq!(input.project.directory.as_deref(), Some("/"));
    assert!(input.input.is_empty());
    assert_eq!(input.character_index, 0);
    assert!(app.err.is_none());
}

#[test]
fn cancel_and_reopen_starts_a_fresh_form() {
    let (mut app, mut input, conn) = setup();
    submit(&mut app, &mut input, &conn, "");
    submit(&mut app, &mut input, &conn, "/");
    press(&mut app, &mut input, &conn, KeyCode::Esc);

    assert!(!app.show_project_input);
    assert!(input.input_mode == InputMode::Normal);
    assert!(app.err.is_none());

    press(&mut app, &mut input, &conn, KeyCode::Char('A'));
    assert!(app.show_project_input);
    assert!(input.input_mode == InputMode::Editing);
    assert!(input.project_step == ProjectStep::Name);
    assert!(input.project.name.is_none());
    assert!(input.project.directory.is_none());
    assert!(input.input.is_empty());
    assert_eq!(input.character_index, 0);
    assert!(app.err.is_none());
}

#[test]
fn typing_during_confirm_does_not_change_the_draft() {
    let (mut app, mut input, conn) = setup();
    submit(&mut app, &mut input, &conn, "Trail");
    submit(&mut app, &mut input, &conn, "/tmp/Trail");
    for key in [
        KeyCode::Char('x'),
        KeyCode::Char('q'),
        KeyCode::Left,
        KeyCode::Backspace,
        KeyCode::Right,
    ] {
        press(&mut app, &mut input, &conn, key);
    }

    assert!(input.project_step == ProjectStep::Confirm);
    assert_eq!(input.project.name.as_deref(), Some("Trail"));
    assert_eq!(input.project.directory.as_deref(), Some("/tmp/Trail"));
    assert!(input.input.is_empty());
    assert_eq!(input.character_index, 0);
    assert!(app.show_project_input);
    assert!(!app.exit);
}

#[test]
fn sqlite_save_persists_project_and_failed_insert_preserves_draft() {
    let (mut app, mut input, conn) = setup();
    conn.execute_batch(
        "CREATE TABLE projects (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            directory TEXT NOT NULL,
            created_at TEXT DEFAULT (datetime('now', 'localtime')),
            updated_at TEXT DEFAULT (datetime('now', 'localtime'))
        );",
    )
    .unwrap();
    submit(&mut app, &mut input, &conn, "Trail");
    submit(&mut app, &mut input, &conn, "/tmp/Trail");
    press(&mut app, &mut input, &conn, KeyCode::Enter);

    let (id, name, directory, created_at, updated_at) = conn
        .query_row(
            "SELECT id, name, directory, created_at, updated_at FROM projects",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .unwrap();
    assert_eq!(uuid::Uuid::parse_str(&id).unwrap().get_version_num(), 4);
    assert_eq!(name, "Trail");
    assert_eq!(directory, "/tmp/Trail");
    assert!(!created_at.is_empty());
    assert!(!updated_at.is_empty());
    assert!(!app.show_project_input);
    assert!(app.err.is_none());
    assert!(input.project.name.is_none());
    assert!(input.project.directory.is_none());

    conn.execute_batch(
        "CREATE TRIGGER fail_insert BEFORE INSERT ON projects
         BEGIN SELECT RAISE(FAIL, 'forced insert failure'); END;",
    )
    .unwrap();
    press(&mut app, &mut input, &conn, KeyCode::Char('A'));
    submit(&mut app, &mut input, &conn, "Another");
    submit(&mut app, &mut input, &conn, "/tmp/Another");
    press(&mut app, &mut input, &conn, KeyCode::Enter);

    assert!(
        app.err
            .as_deref()
            .unwrap()
            .contains("forced insert failure")
    );
    assert!(app.show_project_input);
    assert!(input.input_mode == InputMode::Editing);
    assert!(input.project_step == ProjectStep::Name);
    assert_eq!(input.project.name.as_deref(), Some("Another"));
    assert_eq!(input.project.directory.as_deref(), Some("/tmp/Another"));
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);
}
