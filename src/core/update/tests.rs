use super::*;
use std::{fs, path::PathBuf, process::Command};

struct Fixture {
    dir: PathBuf,
    conn: Connection,
    project: Project,
}

impl Fixture {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!("trail-add-{}", uuid::Uuid::new_v4()));
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

    fn saved_count(&self) -> usize {
        sqlite::get_updates(&self.conn, "p").unwrap().len()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn non_git_update_is_saved_with_null_git_fields_and_default_next() {
    let fixture = Fixture::new();

    let info = add_update(&fixture.conn, &fixture.project, "Title", "Body", None).unwrap();

    assert_eq!((info.title.as_str(), info.body.as_str()), ("Title", "Body"));
    assert_eq!(info.next, "None");
    assert_eq!(info.branch, None);
    assert_eq!(info.commit_sha, None);
    assert_eq!(info.commits_since, None);
}

#[test]
fn git_update_records_branch_and_head() {
    let fixture = Fixture::new();
    fixture.git(&["init", "-q", "-b", "main"]);
    fs::write(fixture.dir.join("a.txt"), "a").unwrap();
    fixture.git(&["add", "a.txt"]);
    fixture.git(&["commit", "-q", "-m", "a"]);
    let head = fixture.git(&["rev-parse", "HEAD"]);

    let info = add_update(&fixture.conn, &fixture.project, "T", "B", Some("Next")).unwrap();

    assert_eq!(info.next, "Next");
    assert_eq!(info.branch.as_deref(), Some("main"));
    assert_eq!(info.commit_sha.as_deref(), Some(head.as_str()));
    assert_eq!(info.commits_since, Some(0));
}

#[test]
fn git_repo_without_commits_records_branch_only() {
    let fixture = Fixture::new();
    fixture.git(&["init", "-q", "-b", "main"]);

    let (branch, commit_sha) = git_context(&fixture.dir);

    assert_eq!(branch.as_deref(), Some("main"));
    assert_eq!(commit_sha, None);
}

#[test]
fn blank_title_or_body_is_rejected_without_saving() {
    let fixture = Fixture::new();

    for (title, body, message) in [
        ("  ", "Body", "title can't be empty"),
        ("Title", "\n", "body can't be empty"),
    ] {
        let error = add_update(&fixture.conn, &fixture.project, title, body, None)
            .err()
            .unwrap();
        assert_eq!(error.to_string(), message);
    }
    assert_eq!(fixture.saved_count(), 0);
}
