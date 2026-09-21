use super::*;
use crate::{
    types::{project::ProjectStep, update::UpdateStep},
    ui::{
        input::{Input, InputMode},
        theme,
    },
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};
use std::{fs, path::PathBuf};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("trail-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn create_project_directory(&self, name: &str) -> String {
        let path = self.0.join(name);
        fs::create_dir(&path).unwrap();
        path.to_str().unwrap().to_owned()
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn setup() -> (App, Input, Connection) {
    (
        App {
            show_project_input: true,
            show_update_input: false,
            show_update_table: false,
            show_help: false,
            projects: Vec::new(),
            updates: Vec::new(),
            project_selection: ListState::default(),
            update_selection: TableState::default(),
            opened_project_id: None,
            opened_update_id: None,
            pending_project_delete_id: None,
            focused_pane: BrowserPane::Projects,
            err: None,
            exit: false,
        },
        Input {
            input_mode: InputMode::Editing,
            ..Input::new()
        },
        Connection::open_in_memory().unwrap(),
    )
}

fn press(app: &mut App, input: &mut Input, conn: &Connection, key: KeyCode) {
    app.handle_key_event(KeyEvent::new(key, KeyModifiers::NONE), input, conn)
        .unwrap();
}

fn focus_projects(app: &mut App, input: &mut Input, conn: &Connection) {
    app.handle_key_event(
        KeyEvent::new(KeyCode::Char('h'), KeyModifiers::CONTROL),
        input,
        conn,
    )
    .unwrap();
}

fn submit(app: &mut App, input: &mut Input, conn: &Connection, text: &str) {
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
    let temp_dir = TestDirectory::new();
    let trail_directory = temp_dir.create_project_directory("Trail");
    let (mut app, mut input, conn) = setup();
    submit(&mut app, &mut input, &conn, "");
    submit(&mut app, &mut input, &conn, &trail_directory);

    assert_eq!(input.project.name.as_deref(), Some("Trail"));
    assert_eq!(
        input.project.directory.as_deref(),
        Some(trail_directory.as_str())
    );
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
    let temp_dir = TestDirectory::new();
    let trail_directory = temp_dir.create_project_directory("Trail");
    let (mut app, mut input, conn) = setup();
    submit(&mut app, &mut input, &conn, "Trail");
    submit(&mut app, &mut input, &conn, &trail_directory);
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
    assert_eq!(
        input.project.directory.as_deref(),
        Some(trail_directory.as_str())
    );
    assert!(input.input.is_empty());
    assert_eq!(input.character_index, 0);
    assert!(app.show_project_input);
    assert!(!app.exit);
}

#[test]
fn sqlite_save_persists_project_and_failed_insert_preserves_draft() {
    let temp_dir = TestDirectory::new();
    let trail_directory = temp_dir.create_project_directory("Trail");
    let (mut app, mut input, conn) = setup();
    sqlite::initialize_schema(&conn).unwrap();
    submit(&mut app, &mut input, &conn, "Trail");
    submit(&mut app, &mut input, &conn, &trail_directory);
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
    assert_eq!(directory, trail_directory);
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
    let another_directory = temp_dir.create_project_directory("Another");
    submit(&mut app, &mut input, &conn, "Another");
    submit(&mut app, &mut input, &conn, &another_directory);
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
    assert_eq!(
        input.project.directory.as_deref(),
        Some(another_directory.as_str())
    );
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);
}

fn browser() -> (App, Input, Connection) {
    let (mut app, mut input, conn) = setup();
    app.show_project_input = false;
    input.input_mode = InputMode::Normal;
    sqlite::initialize_schema(&conn).unwrap();
    (app, input, conn)
}

fn seed_projects(conn: &Connection) {
    conn.execute_batch(
        "INSERT INTO projects (id, name, directory, created_at, updated_at) VALUES
         ('b', 'Beta', '/beta', '2026-09-19 04:00:00', '2026-09-19 04:00:00'),
         ('a', 'Alpha', '/alpha', '2026-09-19 04:00:00', '2026-09-19 04:00:00');",
    )
    .unwrap();
}

fn render_browser(app: &mut App, input: &mut Input) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(100, 12)).unwrap();
    terminal.draw(|frame| app.render(frame, input)).unwrap();
    terminal.backend().buffer().clone()
}

fn screen_text(buffer: &Buffer) -> String {
    buffer.content.iter().map(|cell| cell.symbol()).collect()
}

#[test]
fn empty_browser_ignores_navigation_and_enter() {
    let (mut app, mut input, conn) = browser();
    app.reload_projects(&conn).unwrap();

    for key in [
        KeyCode::Up,
        KeyCode::Down,
        KeyCode::Char('j'),
        KeyCode::Char('k'),
        KeyCode::Enter,
        KeyCode::Char('a'),
    ] {
        press(&mut app, &mut input, &conn, key);
        assert_eq!(app.project_selection.selected(), None);
        assert_eq!(app.opened_project_id, None);
    }
    assert!(!app.show_project_input);
    assert!(!app.show_update_input);
    assert!(!app.exit);
}

#[test]
fn browser_navigation_stops_at_list_boundaries() {
    let (mut app, mut input, conn) = browser();
    seed_projects(&conn);
    app.reload_projects(&conn).unwrap();

    for (key, expected) in [
        (KeyCode::Up, 0),
        (KeyCode::Char('k'), 0),
        (KeyCode::Down, 1),
        (KeyCode::Down, 1),
        (KeyCode::Char('j'), 1),
        (KeyCode::Up, 0),
        (KeyCode::Char('j'), 1),
        (KeyCode::Char('k'), 0),
    ] {
        press(&mut app, &mut input, &conn, key);
        assert_eq!(app.project_selection.selected(), Some(expected));
    }
}

#[test]
fn opened_project_survives_navigation_and_pane_switches() {
    let (mut app, mut input, conn) = browser_with_updates();
    focus_projects(&mut app, &mut input, &conn);
    press(&mut app, &mut input, &conn, KeyCode::Down);
    press(&mut app, &mut input, &conn, KeyCode::Enter);
    focus_projects(&mut app, &mut input, &conn);
    assert_eq!(app.opened_project_id.as_deref(), Some("b"));
    press(&mut app, &mut input, &conn, KeyCode::Up);
    assert_eq!(app.project_selection.selected(), Some(0));
    assert_eq!(app.opened_project_id.as_deref(), Some("b"));

    let buffer = render_browser(&mut app, &mut input);
    assert!(screen_text(&buffer).contains("Body: Beta body"));
    assert_eq!(buffer[(0, 0)].fg, theme::ACCENT);
    assert_eq!(buffer[(20, 0)].fg, theme::BORDER);

    for (key, pane, left, right) in [
        ('l', BrowserPane::LatestUpdate, theme::BORDER, theme::ACCENT),
        ('h', BrowserPane::Projects, theme::ACCENT, theme::BORDER),
    ] {
        app.handle_key_event(
            KeyEvent::new(KeyCode::Char(key), KeyModifiers::CONTROL),
            &mut input,
            &conn,
        )
        .unwrap();
        assert!(app.focused_pane == pane);
        let buffer = render_browser(&mut app, &mut input);
        assert_eq!(buffer[(0, 0)].symbol(), "┌");
        assert_eq!(buffer[(20, 0)].symbol(), "┌");
        assert_eq!(buffer[(0, 0)].fg, left);
        assert_eq!(buffer[(20, 0)].fg, right);
        assert_eq!(app.opened_project_id.as_deref(), Some("b"));

        if app.focused_pane == BrowserPane::LatestUpdate {
            for key in [
                KeyCode::Down,
                KeyCode::Up,
                KeyCode::Char('j'),
                KeyCode::Char('k'),
            ] {
                press(&mut app, &mut input, &conn, key);
                assert_eq!(app.project_selection.selected(), Some(0));
            }
        }
    }
}

#[test]
fn loads_saved_projects_and_refreshes_after_creation() {
    let temp_dir = TestDirectory::new();
    let directory = temp_dir.create_project_directory("New project");
    let (mut app, mut input, conn) = browser();
    seed_projects(&conn);
    app.reload_projects(&conn).unwrap();

    assert_eq!(
        app.projects
            .iter()
            .map(|p| p.id.as_str())
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
    assert_eq!(app.projects[0].name, "Alpha");
    assert_eq!(app.projects[0].directory, "/alpha");
    assert_eq!(
        app.projects[0].created_at.to_rfc3339(),
        "2026-09-19T04:00:00+00:00"
    );
    assert_eq!(app.projects[0].updated_at, app.projects[0].created_at);
    assert_eq!(app.project_selection.selected(), Some(0));

    press(&mut app, &mut input, &conn, KeyCode::Char('A'));
    submit(&mut app, &mut input, &conn, "New project");
    submit(&mut app, &mut input, &conn, &directory);
    press(&mut app, &mut input, &conn, KeyCode::Enter);

    assert_eq!(app.projects.len(), 3);
    let created = app
        .projects
        .iter()
        .find(|p| p.name == "New project")
        .unwrap();
    assert_eq!(created.directory, directory);
    assert!(uuid::Uuid::parse_str(&created.id).is_ok());
    assert!(!app.show_project_input);
    assert!(app.err.is_none());
    assert!(
        app.project_selection
            .selected()
            .is_some_and(|i| i < app.projects.len())
    );
}

#[test]
fn failed_load_preserves_browser_state_and_errors_are_visible() {
    for populated in [false, true] {
        let (mut app, mut input, conn) = browser();
        if populated {
            seed_projects(&conn);
            app.reload_projects(&conn).unwrap();
            press(&mut app, &mut input, &conn, KeyCode::Down);
            press(&mut app, &mut input, &conn, KeyCode::Enter);
        }
        conn.execute_batch("DROP TABLE projects").unwrap();
        let error = app.reload_projects(&conn).unwrap_err();
        assert!(error.to_string().contains("no such table"));
        assert_eq!(app.projects.len(), if populated { 2 } else { 0 });
        assert_eq!(
            app.project_selection.selected(),
            if populated { Some(1) } else { None }
        );
        assert_eq!(
            app.opened_project_id.as_deref(),
            if populated { Some("b") } else { None }
        );

        app.err = Some(format!("Couldn't load projects: {error}"));
        let buffer = render_browser(&mut app, &mut input);
        assert!(screen_text(&buffer).contains(app.err.as_deref().unwrap()));
        let row = if populated { 10 } else { 0 };
        assert_eq!(buffer[(0, row)].fg, theme::ERROR);
    }
}

#[test]
fn successful_save_with_failed_refresh_keeps_error_visible() {
    let temp_dir = TestDirectory::new();
    let directory = temp_dir.create_project_directory("Trail");
    let (mut app, mut input, conn) = browser();
    conn.execute_batch(
        "CREATE TRIGGER invalid_timestamp AFTER INSERT ON projects
         BEGIN UPDATE projects SET updated_at = 'invalid' WHERE id = NEW.id; END;",
    )
    .unwrap();

    press(&mut app, &mut input, &conn, KeyCode::Char('A'));
    submit(&mut app, &mut input, &conn, "Trail");
    submit(&mut app, &mut input, &conn, &directory);
    press(&mut app, &mut input, &conn, KeyCode::Enter);

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM projects", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);
    assert!(!app.show_project_input);
    assert!(input.project.name.is_none());
    assert!(
        app.err
            .as_deref()
            .unwrap()
            .starts_with("Couldn't load projects:")
    );
    let buffer = render_browser(&mut app, &mut input);
    assert!(screen_text(&buffer).contains("Couldn't load projects:"));
    assert_eq!(buffer[(0, 0)].fg, theme::ERROR);
}

#[test]
fn deleting_selected_project_only_closes_it_if_opened() {
    for delete_opened in [false, true] {
        let (mut app, mut input, conn) = browser();
        seed_projects(&conn);
        app.reload_projects(&conn).unwrap();
        press(&mut app, &mut input, &conn, KeyCode::Enter);
        focus_projects(&mut app, &mut input, &conn);
        press(&mut app, &mut input, &conn, KeyCode::Down);
        if delete_opened {
            press(&mut app, &mut input, &conn, KeyCode::Enter);
            focus_projects(&mut app, &mut input, &conn);
        }
        app.err = Some("Previous delete failed".into());

        press(&mut app, &mut input, &conn, KeyCode::Char('D'));
        press(&mut app, &mut input, &conn, KeyCode::Enter);

        let saved = sqlite::get_projects(&conn).unwrap();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].id, "a");
        assert_eq!(app.projects.len(), 1);
        assert_eq!(app.projects[0].id, "a");
        assert_eq!(app.project_selection.selected(), Some(0));
        assert_eq!(
            app.opened_project_id.as_deref(),
            if delete_opened { None } else { Some("a") }
        );
        assert!(app.err.is_none());
    }
}

#[test]
fn deleting_last_project_clears_browser_selection() {
    let (mut app, mut input, conn) = browser();
    seed_projects(&conn);
    conn.execute("DELETE FROM projects WHERE id = 'a'", [])
        .unwrap();
    app.reload_projects(&conn).unwrap();
    press(&mut app, &mut input, &conn, KeyCode::Enter);
    focus_projects(&mut app, &mut input, &conn);

    press(&mut app, &mut input, &conn, KeyCode::Char('D'));
    press(&mut app, &mut input, &conn, KeyCode::Enter);

    assert!(sqlite::get_projects(&conn).unwrap().is_empty());
    assert!(app.projects.is_empty());
    assert_eq!(app.project_selection.selected(), None);
    assert_eq!(app.opened_project_id, None);
    assert!(app.err.is_none());
    assert!(screen_text(&render_browser(&mut app, &mut input)).contains("to create a new project"));

    press(&mut app, &mut input, &conn, KeyCode::Char('D'));
    press(&mut app, &mut input, &conn, KeyCode::Enter);
    assert_eq!(app.project_selection.selected(), None);
    assert!(app.err.is_none());
}

#[test]
fn failed_delete_preserves_browser_and_displays_error() {
    let (mut app, mut input, conn) = browser_with_updates();
    focus_projects(&mut app, &mut input, &conn);
    press(&mut app, &mut input, &conn, KeyCode::Down);
    press(&mut app, &mut input, &conn, KeyCode::Enter);
    focus_projects(&mut app, &mut input, &conn);
    conn.execute_batch(
        "CREATE TRIGGER fail_delete BEFORE DELETE ON projects
         BEGIN SELECT RAISE(FAIL, 'forced delete failure'); END;",
    )
    .unwrap();

    press(&mut app, &mut input, &conn, KeyCode::Char('D'));
    press(&mut app, &mut input, &conn, KeyCode::Enter);

    assert_eq!(sqlite::get_projects(&conn).unwrap().len(), 2);
    assert_eq!(
        app.projects
            .iter()
            .map(|p| p.id.as_str())
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
    assert_eq!(app.project_selection.selected(), Some(1));
    assert_eq!(app.opened_project_id.as_deref(), Some("b"));
    assert!(app.focused_pane == BrowserPane::Projects);
    assert!(!app.show_project_input);
    assert_eq!(
        app.err.as_deref(),
        Some("Unable to delete project: forced delete failure")
    );
    let buffer = render_browser(&mut app, &mut input);
    assert!(screen_text(&buffer).contains(app.err.as_deref().unwrap()));
    assert!(screen_text(&buffer).contains("Body: Beta body"));
    assert_eq!(buffer[(0, 10)].fg, theme::ERROR);
}

#[test]
fn delete_confirmation_blocks_browser_keys_and_escape_cancels() {
    let (mut app, mut input, conn) = browser();
    seed_projects(&conn);
    app.reload_projects(&conn).unwrap();
    press(&mut app, &mut input, &conn, KeyCode::Enter);
    focus_projects(&mut app, &mut input, &conn);
    press(&mut app, &mut input, &conn, KeyCode::Char('D'));

    assert_eq!(app.pending_project_delete_id.as_deref(), Some("a"));
    assert_eq!(sqlite::get_projects(&conn).unwrap().len(), 2);
    let buffer = render_browser(&mut app, &mut input);
    let text = screen_text(&buffer);
    assert!(text.contains("Delete project?"));
    assert!(text.contains("Files on disk will remain."));
    assert!(text.contains("Enter: delete    Esc: cancel"));
    assert_eq!(buffer[(20, 2)].symbol(), "┌");
    assert_eq!(buffer[(21, 3)].symbol(), "A");

    for key in [
        KeyCode::Down,
        KeyCode::Up,
        KeyCode::Char('j'),
        KeyCode::Char('k'),
        KeyCode::Char('A'),
        KeyCode::Char('q'),
        KeyCode::Char('D'),
    ] {
        press(&mut app, &mut input, &conn, key);
    }
    app.handle_key_event(
        KeyEvent::new(KeyCode::Char('l'), KeyModifiers::CONTROL),
        &mut input,
        &conn,
    )
    .unwrap();
    assert_eq!(app.project_selection.selected(), Some(0));
    assert_eq!(app.pending_project_delete_id.as_deref(), Some("a"));
    assert_eq!(app.opened_project_id.as_deref(), Some("a"));
    assert!(app.focused_pane == BrowserPane::Projects);
    assert!(!app.show_project_input);
    assert!(!app.exit);

    press(&mut app, &mut input, &conn, KeyCode::Esc);
    assert!(app.pending_project_delete_id.is_none());
    assert_eq!(sqlite::get_projects(&conn).unwrap().len(), 2);
    assert!(!screen_text(&render_browser(&mut app, &mut input)).contains("Delete project?"));
}

#[test]
fn delete_confirmation_uses_original_id_after_list_reorders() {
    let (mut app, mut input, conn) = browser();
    seed_projects(&conn);
    app.reload_projects(&conn).unwrap();
    press(&mut app, &mut input, &conn, KeyCode::Char('D'));
    app.projects.swap(0, 1);
    press(&mut app, &mut input, &conn, KeyCode::Enter);

    let saved = sqlite::get_projects(&conn).unwrap();
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].id, "b");
    assert!(app.pending_project_delete_id.is_none());
}

fn row_containing(buffer: &Buffer, text: &str) -> u16 {
    (0..buffer.area.height)
        .find(|&y| {
            let row: String = (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect();
            row.contains(text)
        })
        .unwrap_or_else(|| panic!("Text missing from rendered screen: {text}"))
}

#[test]
fn update_details_wrap_body_and_next() {
    let (mut app, mut input, conn) = browser();
    seed_projects(&conn);
    app.reload_projects(&conn).unwrap();
    app.opened_project_id = Some("a".into());
    app.updates.push(Update {
        id: "update-a".into(),
        project_id: "a".into(),
        title: "Progress".into(),
        body: format!("{}BODY_END", "finished task ".repeat(8)),
        next: format!("{}NEXT_END", "another task ".repeat(8)),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    });

    for width in [60, 100] {
        let mut terminal = Terminal::new(TestBackend::new(width, 20)).unwrap();
        terminal
            .draw(|frame| app.render(frame, &mut input))
            .unwrap();
        let buffer = terminal.backend().buffer();

        assert!(row_containing(buffer, "BODY_END") > row_containing(buffer, "Body:"));
        assert!(row_containing(buffer, "What to do next:") > row_containing(buffer, "BODY_END"));
        assert!(row_containing(buffer, "NEXT_END") > row_containing(buffer, "What to do next:"));
        assert!(row_containing(buffer, "Created at:") > row_containing(buffer, "NEXT_END"));
    }
}

#[test]
fn update_confirmation_wraps_body_and_next() {
    let (mut app, mut input, _) = setup();
    app.show_project_input = false;
    app.show_update_input = true;
    app.err = Some("Save failed".into());
    input.update_step = UpdateStep::Confirm;
    input.update.title = Some("Progress".into());
    input.update.body = Some(format!("{}BODY_END", "finished task ".repeat(8)));
    input.update.next = Some(format!("{}NEXT_END", "another task ".repeat(8)));

    for width in [40, 80] {
        let mut terminal = Terminal::new(TestBackend::new(width, 16)).unwrap();
        terminal
            .draw(|frame| app.render(frame, &mut input))
            .unwrap();
        let buffer = terminal.backend().buffer();

        assert!(row_containing(buffer, "BODY_END") > row_containing(buffer, "Body:"));
        assert!(row_containing(buffer, "Next:") > row_containing(buffer, "BODY_END"));
        assert!(row_containing(buffer, "NEXT_END") > row_containing(buffer, "Next:"));
        assert_eq!(row_containing(buffer, "Save failed"), 15);
    }
}

fn browser_with_updates() -> (App, Input, Connection) {
    let (mut app, mut input, conn) = browser();
    seed_projects(&conn);
    conn.execute_batch(
        "INSERT INTO updates (id, project_id, title, body, next, updated_at) VALUES
        ('latest', 'a', 'Latest', 'Latest body', 'Next', '2026-09-20 00:00:00'),
        ('older', 'a', 'Older', 'Older body', 'Next', '2026-09-19 00:00:00'),
        ('beta', 'b', 'Beta progress', 'Beta body', 'Next', '2026-09-19 00:00:00');",
    )
    .unwrap();
    app.reload_projects(&conn).unwrap();
    press(&mut app, &mut input, &conn, KeyCode::Enter);
    (app, input, conn)
}

#[test]
fn opening_selected_update_displays_its_body_without_switching_projects() {
    let (mut app, mut input, conn) = browser_with_updates();
    app.project_selection.select(Some(1));
    app.update_selection.select(Some(1));
    press(&mut app, &mut input, &conn, KeyCode::Char('u'));

    press(&mut app, &mut input, &conn, KeyCode::Enter);

    assert_eq!(app.opened_project_id.as_deref(), Some("a"));
    assert_eq!(app.opened_update_id.as_deref(), Some("older"));
    assert_eq!(app.update_selection.selected(), Some(1));
    assert!(app.focused_pane == BrowserPane::LatestUpdate);
    let text = screen_text(&render_browser(&mut app, &mut input));
    assert!(text.contains("Body: Older body"));
    assert!(!text.contains("Body: Latest body"));

    app.focused_pane = BrowserPane::Projects;
    press(&mut app, &mut input, &conn, KeyCode::Enter);
    assert_eq!(app.opened_project_id.as_deref(), Some("b"));
    assert!(app.opened_update_id.is_none());
    assert_eq!(app.updates.len(), 1);
    assert_eq!(app.updates[0].id, "beta");
    assert!(screen_text(&render_browser(&mut app, &mut input)).contains("Body: Beta body"));
}

#[test]
fn opening_update_without_selection_does_nothing() {
    let (mut app, mut input, conn) = browser_with_updates();
    press(&mut app, &mut input, &conn, KeyCode::Char('u'));
    for selected in [None, Some(10)] {
        app.update_selection.select(selected);
        press(&mut app, &mut input, &conn, KeyCode::Enter);
        assert!(app.opened_update_id.is_none());
        assert!(app.show_update_table);
        assert_eq!(app.opened_project_id.as_deref(), Some("a"));
    }
}

#[test]
fn project_deletion_clears_updates_only_after_deleting_opened_project() {
    for (delete_opened, cancel, fail) in [
        (true, true, false),
        (true, false, true),
        (false, false, false),
        (true, false, false),
    ] {
        let (mut app, mut input, conn) = browser_with_updates();
        focus_projects(&mut app, &mut input, &conn);
        app.opened_update_id = Some("older".into());
        app.update_selection.select(Some(1));
        if !delete_opened {
            app.project_selection.select(Some(1));
        }
        if fail {
            conn.execute_batch(
                "CREATE TRIGGER fail_delete BEFORE DELETE ON projects
                 BEGIN SELECT RAISE(FAIL, 'forced failure'); END;",
            )
            .unwrap();
        }

        press(&mut app, &mut input, &conn, KeyCode::Char('D'));
        assert_eq!(app.updates.len(), 2);
        assert_eq!(app.opened_update_id.as_deref(), Some("older"));
        press(
            &mut app,
            &mut input,
            &conn,
            if cancel { KeyCode::Esc } else { KeyCode::Enter },
        );

        if delete_opened && !cancel && !fail {
            assert!(app.opened_project_id.is_none());
            assert!(app.opened_update_id.is_none());
            assert!(app.updates.is_empty());
            assert_eq!(app.update_selection.selected(), None);
            assert!(sqlite::get_updates(&conn, "a").unwrap().is_empty());
        } else {
            assert_eq!(app.opened_project_id.as_deref(), Some("a"));
            assert_eq!(app.opened_update_id.as_deref(), Some("older"));
            assert_eq!(app.updates.len(), 2);
            assert_eq!(app.update_selection.selected(), Some(1));
            assert!(
                screen_text(&render_browser(&mut app, &mut input)).contains("Body: Older body")
            );
        }
        assert_eq!(app.err.is_some(), fail);
    }
}

#[test]
fn creating_project_clears_previous_update_state() {
    let temp = TestDirectory::new();
    let directory = temp.create_project_directory("New project");
    let (mut app, mut input, conn) = browser_with_updates();
    app.opened_update_id = Some("older".into());
    app.update_selection.select(Some(1));

    press(&mut app, &mut input, &conn, KeyCode::Char('A'));
    submit(&mut app, &mut input, &conn, "New project");
    submit(&mut app, &mut input, &conn, &directory);
    press(&mut app, &mut input, &conn, KeyCode::Enter);

    assert!(app.err.is_none());
    let created = app
        .projects
        .iter()
        .find(|p| p.name == "New project")
        .unwrap();
    assert_eq!(app.opened_project_id.as_deref(), Some(created.id.as_str()));
    assert!(app.updates.is_empty());
    assert!(app.opened_update_id.is_none());
    assert_eq!(app.update_selection.selected(), None);
    assert_eq!(sqlite::get_updates(&conn, "a").unwrap().len(), 2);
}

#[test]
fn create_update_validates_required_fields_and_saves_to_opened_project() {
    for next in ["", "Write tests"] {
        let (mut app, mut input, conn) = browser_with_updates();
        press(&mut app, &mut input, &conn, KeyCode::Down);
        press(&mut app, &mut input, &conn, KeyCode::Char('a'));

        for (step, value) in [
            (UpdateStep::Title, "Progress"),
            (UpdateStep::Body, "Fixed a bug"),
        ] {
            submit(&mut app, &mut input, &conn, "   ");
            assert!(input.update_step == step);
            assert!(app.err.is_some());
            assert!(
                screen_text(&render_browser(&mut app, &mut input))
                    .contains(app.err.as_deref().unwrap())
            );
            submit(&mut app, &mut input, &conn, value);
            assert!(app.err.is_none());
        }
        submit(&mut app, &mut input, &conn, next);
        assert!(input.update_step == UpdateStep::Confirm);
        let confirmation = screen_text(&render_browser(&mut app, &mut input));
        assert!(confirmation.contains("Title: Progress"));
        assert!(confirmation.contains("Body: Fixed a bug"));
        assert_eq!(sqlite::get_updates(&conn, "a").unwrap().len(), 2);

        for key in [KeyCode::Char('q'), KeyCode::Backspace] {
            press(&mut app, &mut input, &conn, key);
        }
        press(&mut app, &mut input, &conn, KeyCode::Enter);

        let saved = sqlite::get_updates(&conn, "a").unwrap();
        assert_eq!(saved.len(), 3);
        let update = saved.iter().find(|u| u.title == "Progress").unwrap();
        assert_eq!(update.body, "Fixed a bug");
        assert_eq!(update.next, if next.is_empty() { "None" } else { next });
        assert_eq!(sqlite::get_updates(&conn, "b").unwrap().len(), 1);
        assert_eq!(app.opened_update_id.as_deref(), Some(update.id.as_str()));
        assert_eq!(
            app.updates[app.update_selection.selected().unwrap()].id,
            update.id
        );
        assert!(!app.show_update_input);
        assert!(!app.exit);
        assert!(app.err.is_none());
        assert!(screen_text(&render_browser(&mut app, &mut input)).contains("Body: Fixed a bug"));
    }
}

#[test]
fn cancel_update_at_each_step_leaves_saved_updates_and_reopens_empty() {
    for completed_fields in 0..=3 {
        let (mut app, mut input, conn) = browser_with_updates();
        press(&mut app, &mut input, &conn, KeyCode::Char('a'));
        for field in ["Draft title", "Draft body", "Draft next"]
            .iter()
            .take(completed_fields)
        {
            submit(&mut app, &mut input, &conn, field);
        }
        press(&mut app, &mut input, &conn, KeyCode::Char('x'));
        press(&mut app, &mut input, &conn, KeyCode::Esc);

        assert!(!app.show_update_input);
        assert_eq!(sqlite::get_updates(&conn, "a").unwrap().len(), 2);
        assert!(screen_text(&render_browser(&mut app, &mut input)).contains("Body: Latest body"));
        press(&mut app, &mut input, &conn, KeyCode::Char('a'));
        assert!(input.update_step == UpdateStep::Title);
        assert!(input.input.is_empty());
        assert_eq!(input.character_index, 0);
        assert!(input.update.title.is_none());
        assert!(input.update.body.is_none());
        assert!(input.update.next.is_none());
        assert!(app.err.is_none());
    }
}

#[test]
fn failed_update_save_preserves_draft_for_retry_without_duplicate_insert() {
    let (mut app, mut input, conn) = browser_with_updates();
    conn.execute_batch(
        "CREATE TRIGGER fail_update BEFORE INSERT ON updates
         BEGIN SELECT RAISE(FAIL, 'forced save failure'); END;",
    )
    .unwrap();
    press(&mut app, &mut input, &conn, KeyCode::Char('a'));
    for field in ["Retry title", "Retry body", "Retry next"] {
        submit(&mut app, &mut input, &conn, field);
    }
    press(&mut app, &mut input, &conn, KeyCode::Enter);

    assert!(app.show_update_input);
    assert!(screen_text(&render_browser(&mut app, &mut input)).contains("forced save failure"));
    assert_eq!(input.update.title.as_deref(), Some("Retry title"));
    assert_eq!(input.update.body.as_deref(), Some("Retry body"));
    assert_eq!(input.update.next.as_deref(), Some("Retry next"));
    assert_eq!(sqlite::get_updates(&conn, "a").unwrap().len(), 2);

    conn.execute_batch("DROP TRIGGER fail_update").unwrap();
    // Keep the preserved fields and confirm the retry.
    for _ in 0..4 {
        press(&mut app, &mut input, &conn, KeyCode::Enter);
    }
    assert!(app.err.is_none());
    assert!(!app.show_update_input);
    let saved = sqlite::get_updates(&conn, "a").unwrap();
    assert_eq!(saved.len(), 3);
    let update = saved.iter().find(|u| u.title == "Retry title").unwrap();
    assert_eq!(update.body, "Retry body");
    assert_eq!(update.next, "Retry next");
}

#[test]
fn update_navigation_uses_update_count_and_stops_at_boundaries() {
    let (mut app, mut input, conn) = browser_with_updates();
    conn.execute("DELETE FROM projects WHERE id = 'b'", [])
        .unwrap();
    app.reload_projects(&conn).unwrap();
    press(&mut app, &mut input, &conn, KeyCode::Char('u'));
    for (key, expected) in [
        (KeyCode::Up, 0),
        (KeyCode::Down, 1),
        (KeyCode::Down, 1),
        (KeyCode::Char('k'), 0),
        (KeyCode::Char('j'), 1),
    ] {
        press(&mut app, &mut input, &conn, key);
        assert_eq!(app.update_selection.selected(), Some(expected));
        assert_eq!(app.project_selection.selected(), Some(0));
    }
}

#[test]
fn update_table_empty_message_does_not_replace_sixth_update() {
    let (mut app, mut input, conn) = browser();
    seed_projects(&conn);
    app.reload_projects(&conn).unwrap();
    app.show_update_table = true;

    let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
    terminal
        .draw(|frame| app.render(frame, &mut input))
        .unwrap();
    assert!(
        screen_text(terminal.backend().buffer())
            .contains("No updates are available for this project.")
    );

    for i in 0..6 {
        app.updates.push(Update {
            id: format!("update-{i}"),
            project_id: "a".into(),
            title: format!("Update {i}"),
            body: format!("Body {i}"),
            next: format!("Next {i}"),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        });
    }

    terminal
        .draw(|frame| app.render(frame, &mut input))
        .unwrap();
    let text = screen_text(terminal.backend().buffer());
    assert!(text.contains("Body 5"));
    assert!(text.contains("Next 5"));
    assert!(!text.to_lowercase().contains("no updates are available"));
}

#[test]
fn update_table_keys_navigate_without_moving_projects() {
    let (mut app, mut input, conn) = browser_with_updates();
    press(&mut app, &mut input, &conn, KeyCode::Char('u'));
    assert!(app.show_update_table);
    for (key, expected) in [
        (KeyCode::Up, 0),
        (KeyCode::Char('j'), 1),
        (KeyCode::Down, 1),
        (KeyCode::Char('k'), 0),
    ] {
        press(&mut app, &mut input, &conn, key);
        assert_eq!(app.update_selection.selected(), Some(expected));
        assert_eq!(app.project_selection.selected(), Some(0));
    }
    for (key, expected) in [
        (KeyCode::Left, 0),
        (KeyCode::Char('l'), 1),
        (KeyCode::Right, 2),
        (KeyCode::Right, 3),
        (KeyCode::Right, 4),
        (KeyCode::Right, 4),
        (KeyCode::Char('h'), 3),
    ] {
        press(&mut app, &mut input, &conn, key);
        assert_eq!(app.update_selection.selected_column(), Some(expected));
    }
    press(&mut app, &mut input, &conn, KeyCode::Char('D'));
    assert!(app.pending_project_delete_id.is_none());
    press(&mut app, &mut input, &conn, KeyCode::Esc);
    assert!(!app.show_update_table);
    assert!(!app.exit);
}

#[test]
fn update_table_enter_opens_selected_update() {
    let (mut app, mut input, conn) = browser_with_updates();
    press(&mut app, &mut input, &conn, KeyCode::Char('u'));
    press(&mut app, &mut input, &conn, KeyCode::Down);
    press(&mut app, &mut input, &conn, KeyCode::Enter);
    assert!(!app.show_update_table);
    assert_eq!(app.opened_update_id.as_deref(), Some("older"));
    assert_eq!(app.opened_project_id.as_deref(), Some("a"));
    assert!(app.focused_pane == BrowserPane::LatestUpdate);
    assert!(screen_text(&render_browser(&mut app, &mut input)).contains("Body: Older body"));
}

#[test]
fn update_table_empty_navigation_and_quit() {
    let (mut app, mut input, conn) = browser();
    seed_projects(&conn);
    app.reload_projects(&conn).unwrap();
    press(&mut app, &mut input, &conn, KeyCode::Enter);
    press(&mut app, &mut input, &conn, KeyCode::Char('u'));
    for key in [
        KeyCode::Up,
        KeyCode::Down,
        KeyCode::Left,
        KeyCode::Right,
        KeyCode::Enter,
    ] {
        press(&mut app, &mut input, &conn, key);
        assert!(app.show_update_table);
        assert_eq!(app.update_selection.selected(), None);
        assert_eq!(app.update_selection.selected_column(), None);
        assert!(app.opened_update_id.is_none());
    }
    press(&mut app, &mut input, &conn, KeyCode::Char('q'));
    assert!(app.exit);
}

#[test]
fn browser_focus_stays_on_visible_panes() {
    let (mut app, mut input, conn) = browser_with_updates();
    let selected = app.update_selection.selected();
    for _ in 0..2 {
        app.handle_key_event(
            KeyEvent::new(KeyCode::Char('l'), KeyModifiers::CONTROL),
            &mut input,
            &conn,
        )
        .unwrap();
        assert!(app.focused_pane == BrowserPane::LatestUpdate);
    }
    press(&mut app, &mut input, &conn, KeyCode::Char('j'));
    assert_eq!(app.update_selection.selected(), selected);
    press(&mut app, &mut input, &conn, KeyCode::Char('u'));
    press(&mut app, &mut input, &conn, KeyCode::Esc);
    assert!(app.focused_pane == BrowserPane::LatestUpdate);
    for _ in 0..2 {
        focus_projects(&mut app, &mut input, &conn);
        assert!(app.focused_pane == BrowserPane::Projects);
    }
}

#[test]
fn update_table_scrollbar_tracks_viewport_without_covering_dates() {
    let (mut app, mut input, conn) = browser_with_updates();
    press(&mut app, &mut input, &conn, KeyCode::Char('u'));
    for i in 2..8 {
        app.updates.push(Update {
            id: format!("update-{i}"),
            project_id: "a".into(),
            title: format!("Update {i}"),
            body: format!("Body {i}"),
            next: format!("Next {i}"),
            created_at: app.updates[0].created_at,
            updated_at: app.updates[0].updated_at,
        });
    }
    let mut terminal = Terminal::new(TestBackend::new(120, 21)).unwrap();
    terminal
        .draw(|frame| app.render(frame, &mut input))
        .unwrap();
    assert_eq!(app.update_selection.offset(), 0);
    let buffer = terminal.backend().buffer();
    // Four of eight rows fit. The thumb fills half the sixteen-line track.
    for y in 1..17 {
        assert_eq!(buffer[(119, y)].symbol(), if y <= 8 { "█" } else { "║" });
    }
    assert_eq!(buffer[(118, 2)].symbol(), ":");

    for _ in 0..7 {
        press(&mut app, &mut input, &conn, KeyCode::Down);
    }
    terminal
        .draw(|frame| app.render(frame, &mut input))
        .unwrap();
    assert_eq!(app.update_selection.offset(), 4);
    let buffer = terminal.backend().buffer();
    for y in 1..17 {
        assert_eq!(buffer[(119, y)].symbol(), if y <= 8 { "║" } else { "█" });
    }

    // Resizing to fit every row removes the scrollbar without another key event.
    terminal.backend_mut().resize(120, 40);
    terminal.autoresize().unwrap();
    terminal
        .draw(|frame| app.render(frame, &mut input))
        .unwrap();
    assert_eq!(app.update_selection.offset(), 0);
    for y in 1..36 {
        assert_eq!(terminal.backend().buffer()[(119, y)].symbol(), " ");
    }

    // Reloading a smaller history must not retain the previous scroll position.
    app.reload_updates(&conn).unwrap();
    terminal.backend_mut().resize(120, 21);
    terminal.autoresize().unwrap();
    terminal
        .draw(|frame| app.render(frame, &mut input))
        .unwrap();
    assert_eq!(app.update_selection.offset(), 0);
    for y in 1..17 {
        assert_eq!(terminal.backend().buffer()[(119, y)].symbol(), " ");
    }
}

#[test]
fn help_lists_controls_and_blocks_browser_actions() {
    let (mut app, mut input, conn) = browser_with_updates();
    press(&mut app, &mut input, &conn, KeyCode::Char('?'));
    let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
    terminal
        .draw(|frame| app.render(frame, &mut input))
        .unwrap();
    let text = screen_text(terminal.backend().buffer());
    for label in [
        "Trail keybindings",
        "Browser",
        "Update table",
        "Project and update forms",
        "Delete confirmation",
        "Backspace",
        "Ctrl+h",
        "Esc / ?",
    ] {
        assert!(text.contains(label), "Missing help: {label}");
    }
    for key in [
        KeyCode::Char('A'),
        KeyCode::Char('a'),
        KeyCode::Char('D'),
        KeyCode::Char('u'),
        KeyCode::Down,
        KeyCode::Enter,
    ] {
        press(&mut app, &mut input, &conn, key);
    }
    assert!(app.show_help);
    assert!(!app.show_project_input);
    assert!(!app.show_update_input);
    assert!(!app.show_update_table);
    assert!(app.pending_project_delete_id.is_none());
    assert_eq!(app.project_selection.selected(), Some(0));
    for close in [KeyCode::Esc, KeyCode::Char('?')] {
        app.show_help = true;
        press(&mut app, &mut input, &conn, close);
        assert!(!app.show_help);
        assert!(!app.exit);
        assert_eq!(app.opened_project_id.as_deref(), Some("a"));
    }
    app.show_help = true;
    press(&mut app, &mut input, &conn, KeyCode::Char('q'));
    assert!(app.exit);
}

#[test]
fn input_forms_show_placeholders_centered_with_help_below() {
    let (mut app, mut input, _) = setup();
    let mut terminal = Terminal::new(TestBackend::new(100, 24)).unwrap();
    for (project, project_step, update_step, placeholder) in [
        (true, ProjectStep::Name, UpdateStep::Title, "Optional: name"),
        (
            true,
            ProjectStep::Directory,
            UpdateStep::Title,
            "Required: path",
        ),
        (
            false,
            ProjectStep::Name,
            UpdateStep::Title,
            "Required: a short title",
        ),
        (
            false,
            ProjectStep::Name,
            UpdateStep::Body,
            "Required: what did",
        ),
        (
            false,
            ProjectStep::Name,
            UpdateStep::Next,
            "Optional: what will",
        ),
    ] {
        app.show_project_input = project;
        app.show_update_input = !project;
        input.project_step = project_step;
        input.update_step = update_step;
        terminal
            .draw(|frame| app.render(frame, &mut input))
            .unwrap();
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(14, 10)].symbol(), "┌");
        assert_eq!(buffer[(85, 12)].symbol(), "┘");
        assert!(screen_text(buffer).contains(placeholder));
        let help: String = (14..86).map(|x| buffer[(x, 13)].symbol()).collect();
        assert!(help.contains("Enter: continue | Esc: cancel"));
        assert_eq!(
            help.contains("Shift+Enter"),
            !project && input.update_step != UpdateStep::Title
        );
        assert_eq!(buffer[(15, 11)].fg, theme::MUTED);
    }
}

#[test]
fn input_wraps_grows_and_scrolls_with_the_cursor() {
    use ratatui::{backend::Backend, layout::Position};
    let (mut app, mut input, _) = setup();
    let mut terminal = Terminal::new(TestBackend::new(100, 24)).unwrap();
    input.input = format!("{}界e\u{301}Z", "x".repeat(69));
    input.character_index = input.input.chars().count();
    terminal
        .draw(|frame| app.render(frame, &mut input))
        .unwrap();
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(14, 10)].symbol(), "┌");
    assert_eq!(buffer[(85, 13)].symbol(), "┘");
    assert_eq!(buffer[(15, 12)].symbol(), "界");
    assert_eq!(buffer[(17, 12)].symbol(), "e\u{301}");
    assert_eq!(
        terminal.backend_mut().get_cursor_position().unwrap(),
        Position::new(19, 12)
    );
    input.move_cursor_left();
    input.delete_char();
    terminal
        .draw(|frame| app.render(frame, &mut input))
        .unwrap();
    assert_eq!(
        terminal.backend_mut().get_cursor_position().unwrap(),
        Position::new(18, 12)
    );

    input.input = "line\n".repeat(40);
    input.character_index = input.input.chars().count();
    for (width, height) in [(100, 24), (20, 8), (3, 5)] {
        terminal.backend_mut().resize(width, height);
        terminal.autoresize().unwrap();
        terminal
            .draw(|frame| app.render(frame, &mut input))
            .unwrap();
        let cursor = terminal.backend_mut().get_cursor_position().unwrap();
        assert!(cursor.x > 0 && cursor.x < width - 1);
        assert!(cursor.y > 0 && cursor.y < height - 1);
        input.character_index = 0;
        terminal
            .draw(|frame| app.render(frame, &mut input))
            .unwrap();
        assert!(screen_text(terminal.backend().buffer()).contains('l'));
        input.character_index = input.input.chars().count();
    }
}

#[test]
fn shift_enter_adds_newlines_only_to_body_and_next_and_saves_them() {
    let (mut app, mut input, conn) = browser_with_updates();
    press(&mut app, &mut input, &conn, KeyCode::Char('a'));
    input.input = "Multiline".into();
    input.character_index = input.input.chars().count();
    app.handle_key_event(
        KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT),
        &mut input,
        &conn,
    )
    .unwrap();
    assert!(input.update_step == UpdateStep::Body);
    for step in [UpdateStep::Body, UpdateStep::Next] {
        input.input = "firstsecond".into();
        input.character_index = 5;
        app.handle_key_event(
            KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT),
            &mut input,
            &conn,
        )
        .unwrap();
        assert!(input.update_step == step);
        assert_eq!(input.input, "first\nsecond");
        assert_eq!(input.character_index, 6);
        press(&mut app, &mut input, &conn, KeyCode::Enter);
    }
    let confirmation = render_browser(&mut app, &mut input);
    for y in [3, 5] {
        let line: String = (0..6).map(|x| confirmation[(x, y)].symbol()).collect();
        assert_eq!(line, "second");
    }
    press(&mut app, &mut input, &conn, KeyCode::Enter);
    let updates = sqlite::get_updates(&conn, "a").unwrap();
    let saved = updates.iter().find(|u| u.title == "Multiline").unwrap();
    assert_eq!(saved.body, "first\nsecond");
    assert_eq!(saved.next, "first\nsecond");

    let saved_view = render_browser(&mut app, &mut input);
    for y in [3, 5] {
        let line: String = (21..27).map(|x| saved_view[(x, y)].symbol()).collect();
        assert_eq!(line, "second");
    }

    press(&mut app, &mut input, &conn, KeyCode::Char('A'));
    app.handle_key_event(
        KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT),
        &mut input,
        &conn,
    )
    .unwrap();
    assert!(input.project_step == ProjectStep::Directory);
    assert!(input.input.is_empty());
}

#[test]
fn theme_distinguishes_table_row_column_and_cell_after_navigation() {
    use ratatui::style::Modifier;
    let (mut app, mut input, conn) = browser_with_updates();
    press(&mut app, &mut input, &conn, KeyCode::Char('u'));
    app.update_selection.select(Some(0));
    app.update_selection.select_column(Some(0));
    let before = render_browser(&mut app, &mut input);
    for style in [theme::ROW, theme::COLUMN, theme::CELL] {
        assert!(before.content.iter().any(|cell| Some(cell.bg) == style.bg));
    }
    assert!(
        before
            .content
            .iter()
            .all(|cell| !cell.modifier.contains(Modifier::REVERSED))
    );
    assert_eq!(before[(3, 2)].bg, theme::ACCENT);
    assert_eq!(before[(3, 2)].fg, theme::BACKGROUND);
    assert!(before[(3, 2)].modifier.contains(Modifier::BOLD));
    assert_eq!(before[(3, 0)].bg, theme::ACCENT);

    press(&mut app, &mut input, &conn, KeyCode::Right);
    press(&mut app, &mut input, &conn, KeyCode::Down);
    let after = render_browser(&mut app, &mut input);
    assert_eq!(after[(3, 2)].bg, theme::BACKGROUND);
    assert_eq!(after[(3, 0)].fg, theme::ACCENT);
    assert_ne!(after[(3, 0)].bg, theme::ACCENT);
    for style in [theme::ROW, theme::COLUMN, theme::CELL] {
        assert!(after.content.iter().any(|cell| Some(cell.bg) == style.bg));
    }
}

#[test]
fn theme_covers_focus_empty_panes_forms_and_delete_popup() {
    let (mut app, mut input, conn) = browser_with_updates();
    focus_projects(&mut app, &mut input, &conn);
    let focused = render_browser(&mut app, &mut input);
    assert_eq!(focused[(2, 1)].fg, theme::ACCENT);
    assert_eq!(Some(focused[(2, 1)].bg), theme::ROW.bg);
    app.focused_pane = BrowserPane::LatestUpdate;
    app.updates.clear();
    let empty = render_browser(&mut app, &mut input);
    assert_eq!(empty[(2, 1)].fg, theme::MUTED);
    assert_eq!(empty[(20, 0)].fg, theme::ACCENT);
    assert_eq!(empty[(21, 1)].fg, theme::MUTED);
    assert_eq!(empty[(99, 11)].bg, theme::BACKGROUND);

    focus_projects(&mut app, &mut input, &conn);
    press(&mut app, &mut input, &conn, KeyCode::Char('D'));
    let popup = render_browser(&mut app, &mut input);
    assert_eq!(popup[(20, 2)].fg, theme::ERROR);
    assert_eq!(popup[(21, 3)].fg, theme::TEXT);
    assert_eq!(popup[(21, 3)].bg, theme::BACKGROUND);
    press(&mut app, &mut input, &conn, KeyCode::Esc);
    press(&mut app, &mut input, &conn, KeyCode::Char('A'));
    let form = render_browser(&mut app, &mut input);
    assert_eq!(form[(14, 4)].fg, theme::ACCENT);
    assert_eq!(form[(15, 5)].fg, theme::MUTED);
    press(&mut app, &mut input, &conn, KeyCode::Char('X'));
    let typed = render_browser(&mut app, &mut input);
    assert_eq!(typed[(15, 5)].fg, theme::TEXT);
    assert_eq!(typed[(15, 5)].bg, theme::BACKGROUND);
}
