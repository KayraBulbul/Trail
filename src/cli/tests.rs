use super::*;
use clap::CommandFactory;

fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
    Cli::try_parse_from(std::iter::once("trail").chain(args.iter().copied()))
}

#[test]
fn definitions_are_valid() {
    Cli::command().debug_assert();
}

#[test]
fn no_command_opens_the_tui() {
    let cli = parse(&[]).unwrap();
    assert!(cli.command.is_none());
    assert!(!cli.version);
}

#[test]
fn version_flag_keeps_short_lowercase_v() {
    assert!(parse(&["-v"]).unwrap().version);
    assert!(parse(&["--version"]).unwrap().version);
}

#[test]
fn log_defaults_to_five_and_json_works_after_the_command() {
    let cli = parse(&["log", "--json"]).unwrap();
    assert!(cli.json);
    let Some(Command::Log { count, target }) = cli.command else {
        panic!("expected log");
    };
    assert_eq!(count, 5);
    assert_eq!(target.project, None);
}

#[test]
fn add_requires_a_body_and_takes_a_short_project_flag() {
    assert!(parse(&["add", "--title", "T"]).is_err());

    let cli = parse(&["add", "-p", "Trail", "--title", "T", "--body", "B"]).unwrap();
    let Some(Command::Add { target, .. }) = cli.command else {
        panic!("expected add");
    };
    assert_eq!(target.project.as_deref(), Some("Trail"));
}

struct Folder(std::path::PathBuf);

impl Folder {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("trail-cli-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(path.join("sub")).unwrap();
        Folder(path)
    }
}

impl Drop for Folder {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn database() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    sqlite::initialize_schema(&conn).unwrap();
    conn
}

/// Parses `args` like the real binary and runs them from `cwd` with `stdin`.
fn trail(conn: &Connection, cwd: &Path, args: &[&str], stdin: &str) -> Result<String, String> {
    let cli = parse(args).unwrap();
    execute(
        conn,
        cwd,
        cli.command.unwrap(),
        cli.json,
        &mut stdin.as_bytes(),
    )
    .map_err(|error| error_message(&error))
}

#[test]
fn commands_without_a_project_point_to_init() {
    let (conn, folder) = (database(), Folder::new());

    for args in [
        &["status"][..],
        &["log"],
        &["add", "--title", "T", "--body", "B"],
    ] {
        let error = trail(&conn, &folder.0, args, "").unwrap_err();
        assert!(error.contains("Run `trail init`"), "{error}");
    }
    let error = trail(&conn, &folder.0, &["status", "-p", "nope"], "").unwrap_err();
    assert!(error.contains("trail projects"), "{error}");
}

#[test]
fn init_add_status_and_log_from_a_subfolder() {
    let (conn, folder) = (database(), Folder::new());
    let sub = folder.0.join("sub");

    let created = trail(&conn, &folder.0, &["init", "--name", "Demo"], "").unwrap();
    assert!(created.starts_with("Created project Demo at "));
    assert!(
        trail(&conn, &sub, &["status"], "")
            .unwrap()
            .contains("No updates yet")
    );

    let added = trail(
        &conn,
        &sub,
        &["add", "--title", "First", "--body", "-"],
        "Line one\n\nLine two\n\n",
    )
    .unwrap();
    assert_eq!(added, "Added to Demo: First");
    trail(
        &conn,
        &sub,
        &[
            "add", "--title", "Second", "--body", "B", "--next", "Ship it",
        ],
        "",
    )
    .unwrap();

    let status = trail(&conn, &sub, &["status"], "").unwrap();
    assert!(status.starts_with("Demo  "), "{status}");
    assert!(!status.contains("Git"), "{status}");
    assert!(status.contains("Latest update  (just now)\n  Second\n  B\nNext\n  Ship it"));

    let log = trail(&conn, &sub, &["log", "-n", "1"], "").unwrap();
    assert!(log.starts_with("Second  (just now)"), "{log}");
    assert!(!log.contains("First"), "{log}");
    let log = trail(&conn, &sub, &["log"], "").unwrap();
    assert!(
        log.contains("First  (just now)\n  Line one\n\n  Line two\n  Next: None"),
        "{log}"
    );
}

#[test]
fn json_flag_switches_each_command_to_json() {
    let (conn, folder) = (database(), Folder::new());
    trail(&conn, &folder.0, &["init", "--name", "Demo"], "").unwrap();

    let status: serde_json::Value =
        serde_json::from_str(&trail(&conn, &folder.0, &["status", "--json"], "").unwrap()).unwrap();
    assert_eq!(status["project"]["name"], "Demo");

    let added: serde_json::Value = serde_json::from_str(
        &trail(
            &conn,
            &folder.0,
            &["add", "--json", "--title", "T", "--body", "B"],
            "",
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(added["title"], "T");

    let projects: serde_json::Value =
        serde_json::from_str(&trail(&conn, &folder.0, &["projects", "--json"], "").unwrap())
            .unwrap();
    assert_eq!(projects[0]["update_count"], 1);
    assert_eq!(projects[0]["is_git"], false);
    assert!(projects[0]["last_update_at"].is_string());
}

#[test]
fn init_inside_a_project_prints_a_note() {
    let (conn, folder) = (database(), Folder::new());
    trail(&conn, &folder.0, &["init", "--name", "Outer"], "").unwrap();

    let created = trail(&conn, &folder.0.join("sub"), &["init"], "").unwrap();

    assert!(
        created.ends_with("Note: this folder is inside Outer. Commands run here will now use sub.")
    );
}

#[test]
fn agents_snippet_covers_the_session_workflow() {
    for text in [
        "trail status",
        "trail add",
        "--body -",
        "--json",
        "Never edit or delete",
    ] {
        assert!(AGENTS_SNIPPET.contains(text), "missing {text}");
    }
}
