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

impl Fixture {
    /// Runs `init_project` from `dir`; the created project's name and `inside`, or the error message.
    fn init(&self, dir: &str, name: Option<&str>) -> Result<(String, Option<String>), String> {
        init_project(&self.conn, &self.root.join(dir), name)
            .map(|init| (init.project.name, init.inside))
            .map_err(|error| error.to_string())
    }
}

#[test]
fn init_names_the_project_after_the_folder_and_saves_its_real_path() {
    let fixture = Fixture::new();

    assert_eq!(
        fixture.init("trail/src/../", None),
        Ok(("trail".into(), None))
    );

    let saved = &sqlite::get_projects(&fixture.conn).unwrap()[0];
    let real = fs::canonicalize(fixture.root.join("trail")).unwrap();
    assert_eq!(saved.directory, real.to_str().unwrap());
}

#[test]
fn init_refuses_the_same_folder_twice() {
    let fixture = Fixture::new();
    fixture.add("Trail", &format!("{}/", fixture.path("trail")));

    assert_eq!(
        fixture.init("trail", Some("Again")),
        Err("Trail is already a project".into())
    );
    assert_eq!(sqlite::get_projects(&fixture.conn).unwrap().len(), 1);
}

#[test]
fn init_inside_another_project_reports_it() {
    let fixture = Fixture::new();
    fixture.add("Trail", &fixture.path("trail"));

    assert_eq!(
        fixture.init("trail/nested", None),
        Ok(("nested".into(), Some("Trail".into())))
    );
}

#[test]
fn init_rejects_blank_and_taken_names() {
    let fixture = Fixture::new();
    fixture.add("Trail", &fixture.path("trail"));

    assert_eq!(
        fixture.init("trail2", Some("   ")),
        Err("project name can't be empty".into())
    );
    assert_eq!(
        fixture.init("trail2", Some("TRAIL")),
        Err("a project named TRAIL already exists, pick another with --name".into())
    );
    assert_eq!(sqlite::get_projects(&fixture.conn).unwrap().len(), 1);
}

#[test]
fn relative_saved_directories_never_match() {
    let fixture = Fixture::new();
    fixture.add("Dot", ".");
    fixture.add("Home", "~/trail");

    let here = std::env::current_dir().unwrap();
    let error = resolve_project(&fixture.conn, &here, None).err().unwrap();

    assert_eq!(error.to_string(), "no project covers this folder");
    assert_eq!(fixture.resolve("trail", Some("dot")), Ok("Dot".into()));
}
