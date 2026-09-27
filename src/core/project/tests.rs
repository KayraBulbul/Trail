use super::*;
use crate::types::project::ProjectDraft;
use std::{fs, path::PathBuf};

struct Fixture {
    root: PathBuf,
    conn: Connection,
}

impl Fixture {
    /// Folders `trail/src/deep`, `trail/nested/inner` and `trail2` under a temp root.
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("trail-core-{}", uuid::Uuid::new_v4()));
        for dir in ["trail/src/deep", "trail/nested/inner", "trail2"] {
            fs::create_dir_all(root.join(dir)).unwrap();
        }
        let conn = Connection::open_in_memory().unwrap();
        sqlite::initialize_schema(&conn).unwrap();
        Fixture { root, conn }
    }

    fn add(&self, name: &str, directory: &str) {
        let draft = ProjectDraft {
            name: Some(name.to_string()),
            directory: Some(directory.to_string()),
        };
        sqlite::insert_project(&self.conn, &draft).unwrap();
    }

    fn path(&self, dir: &str) -> String {
        self.root.join(dir).to_str().unwrap().to_string()
    }

    /// The resolved project's name, or the error message.
    fn resolve(&self, dir: &str, name: Option<&str>) -> Result<String, String> {
        resolve_project(&self.conn, &self.root.join(dir), name)
            .map(|project| project.name)
            .map_err(|error| error.to_string())
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn matches_the_project_folder_and_its_subfolders() {
    let fixture = Fixture::new();
    fixture.add("Trail", &fixture.path("trail"));

    assert_eq!(fixture.resolve("trail", None), Ok("Trail".into()));
    assert_eq!(fixture.resolve("trail/src/deep", None), Ok("Trail".into()));
}

#[test]
fn deepest_project_wins() {
    let fixture = Fixture::new();
    fixture.add("Nested", &fixture.path("trail/nested"));
    fixture.add("Trail", &fixture.path("trail"));

    assert_eq!(
        fixture.resolve("trail/nested/inner", None),
        Ok("Nested".into())
    );
    assert_eq!(fixture.resolve("trail/src", None), Ok("Trail".into()));
}

#[test]
fn sibling_with_a_shared_prefix_does_not_match() {
    let fixture = Fixture::new();
    fixture.add("Trail", &fixture.path("trail"));

    assert_eq!(
        fixture.resolve("trail2", None),
        Err("no project covers this folder".into())
    );
}

#[test]
fn saved_directories_are_normalised_and_missing_ones_skipped() {
    let fixture = Fixture::new();
    fixture.add("Gone", &fixture.path("deleted"));
    fixture.add("Trail", &format!("{}/", fixture.path("trail/../trail")));

    assert_eq!(fixture.resolve("trail/src", None), Ok("Trail".into()));
}

#[test]
fn name_override_ignores_case_and_the_current_folder() {
    let fixture = Fixture::new();
    fixture.add("Trail", &fixture.path("trail"));
    fixture.add("Other", &fixture.path("trail2"));

    assert_eq!(fixture.resolve("trail2", Some("TRAIL")), Ok("Trail".into()));
    assert_eq!(
        fixture.resolve("trail", Some("missing")),
        Err("no project has that name".into())
    );
}
