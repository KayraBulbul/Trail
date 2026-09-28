use super::*;
use std::{fs, path::PathBuf, process::Command};

struct Fixture {
    dir: PathBuf,
    conn: Connection,
    project: Project,
}

impl Fixture {
    /// A project in an empty temp folder (not a git repo yet).
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("trail-status-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let conn = Connection::open_in_memory().unwrap();
        sqlite::initialize_schema(&conn).unwrap();
        conn.execute(
            "INSERT INTO projects (id, name, directory) VALUES ('p', 'Trail', ?1)",
            [dir.to_str().unwrap()],
        )
        .unwrap();
        let project = sqlite::get_projects(&conn).unwrap().remove(0);
        Fixture { dir, conn, project }
    }

    fn git(&self, args: &[&str]) -> String {
        let output = Command::new("git")
            .arg("-C")
            .arg(&self.dir)
            .args([
                "-c",
                "user.name=Trail Test",
                "-c",
                "user.email=test@trail.invalid",
            ])
            .args(["-c", "commit.gpgsign=false"])
            .args(args)
            .output()
            .unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap().trim().to_string()
    }

    fn commit(&self, name: &str) -> String {
        fs::write(self.dir.join(name), name).unwrap();
        self.git(&["add", name]);
        self.git(&["commit", "-q", "-m", name]);
        self.git(&["rev-parse", "HEAD"])
    }

    /// Inserts an update with a fixed timestamp so ordering is deterministic.
    fn update(&self, id: &str, updated_at: &str, commit_sha: Option<&str>) {
        self.conn
            .execute(
                "INSERT INTO updates (id, project_id, title, body, next, created_at, updated_at, branch, commit_sha)
                 VALUES (?1, 'p', ?1, 'body', 'next', ?2, ?2, 'main', ?3)",
                (id, updated_at, commit_sha),
            )
            .unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn non_git_project_without_updates_has_nulls() {
    let fixture = Fixture::new();

    let status = status(&fixture.conn, &fixture.project).unwrap();

    assert_eq!(status.project.name, "Trail");
    assert!(status.git.is_none());
    assert!(status.latest_update.is_none());
    let json = serde_json::to_value(&status).unwrap();
    assert_eq!(json["git"], serde_json::Value::Null);
    assert_eq!(json["latest_update"], serde_json::Value::Null);
}

#[test]
fn status_reports_git_and_commits_since_the_latest_update() {
    let fixture = Fixture::new();
    fixture.git(&["init", "-q", "-b", "main"]);
    let written_at = fixture.commit("a.txt");
    fixture.update("latest", "2026-09-27 10:00:00", Some(&written_at));
    fixture.commit("b.txt");
    fixture.commit("c.txt");
    fs::write(fixture.dir.join("dirty.txt"), "x").unwrap();

    let status = status(&fixture.conn, &fixture.project).unwrap();

    let git = status.git.unwrap();
    assert_eq!(git.branch.as_deref(), Some("main"));
    assert_eq!(git.files_changed, 1);
    assert!(git.last_commit.is_some());
    let latest = status.latest_update.unwrap();
    assert_eq!(latest.id, "latest");
    assert_eq!(latest.commits_since, Some(2));
}

#[test]
fn log_is_newest_first_limited_and_unknown_commits_are_null() {
    let fixture = Fixture::new();
    fixture.git(&["init", "-q", "-b", "main"]);
    fixture.commit("a.txt");
    fixture.update("old", "2026-09-25 10:00:00", None);
    fixture.update("mid", "2026-09-26 10:00:00", Some(&"0".repeat(40)));
    fixture.update("new", "2026-09-27 10:00:00", None);

    let log = log(&fixture.conn, &fixture.project, 2).unwrap();

    let ids: Vec<&str> = log.iter().map(|update| update.id.as_str()).collect();
    assert_eq!(ids, ["new", "mid"]);
    assert_eq!(log[1].commits_since, None);
    let json = serde_json::to_value(&log[0]).unwrap();
    assert_eq!(json["commit_sha"], serde_json::Value::Null);
    assert_eq!(json["commits_since"], serde_json::Value::Null);
    assert_eq!(json["created_at"], "2026-09-27T10:00:00Z");
}
