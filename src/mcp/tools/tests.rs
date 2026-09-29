use super::*;
use crate::database::sqlite;
use std::{fs, path::PathBuf};

struct Fixture {
    root: PathBuf,
    conn: Connection,
}

impl Fixture {
    /// A `Demo` project in `root/demo`; `root/elsewhere` belongs to no project.
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("trail-mcp-{}", uuid::Uuid::new_v4()));
        for dir in ["demo", "elsewhere"] {
            fs::create_dir_all(root.join(dir)).unwrap();
        }
        let conn = Connection::open_in_memory().unwrap();
        sqlite::initialize_schema(&conn).unwrap();
        conn.execute(
            "INSERT INTO projects (id, name, directory) VALUES ('d', 'Demo', ?1)",
            [root.join("demo").to_str().unwrap()],
        )
        .unwrap();
        Fixture { root, conn }
    }

    /// Calls a tool from `dir`; the result text and whether it's an error.
    fn call(&self, dir: &str, name: &str, arguments: Value) -> (String, bool) {
        let result = call(&self.conn, &self.root.join(dir), name, &arguments).unwrap();
        (
            result["content"][0]["text"].as_str().unwrap().to_string(),
            result["isError"].as_bool().unwrap(),
        )
    }

    /// Calls a tool that should succeed and parses its JSON text.
    fn json(&self, dir: &str, name: &str, arguments: Value) -> Value {
        let (text, is_error) = self.call(dir, name, arguments);
        assert!(!is_error, "{text}");
        serde_json::from_str(&text).unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn definitions_list_the_four_tools_with_object_schemas() {
    let tools = definitions();
    let names: Vec<&str> = tools
        .as_array()
        .unwrap()
        .iter()
        .map(|tool| tool["name"].as_str().unwrap())
        .collect();

    assert_eq!(
        names,
        ["trail_status", "trail_log", "trail_add", "trail_projects"]
    );
    for tool in tools.as_array().unwrap() {
        assert_eq!(tool["inputSchema"]["type"], "object");
        assert!(tool["description"].is_string());
    }
    assert_eq!(
        tools[2]["inputSchema"]["required"],
        json!(["title", "body"])
    );
}

#[test]
fn add_then_status_and_log_return_the_cli_json() {
    let fixture = Fixture::new();

    let added = fixture.json(
        "demo",
        "trail_add",
        json!({ "title": "First", "body": "Did it" }),
    );
    assert_eq!(added["next"], "None");

    let status = fixture.json("demo", "trail_status", json!({}));
    assert_eq!(status["project"]["name"], "Demo");
    assert_eq!(status["latest_update"]["title"], "First");

    fixture.json(
        "demo",
        "trail_add",
        json!({ "title": "Second", "body": "B", "next": "Ship" }),
    );
    let log = fixture.json("demo", "trail_log", json!({ "count": 1 }));
    assert_eq!(log.as_array().unwrap().len(), 1);
    assert_eq!(log[0]["title"], "Second");
}

#[test]
fn missing_arguments_count_as_empty_and_projects_lists_counts() {
    let fixture = Fixture::new();

    let projects = fixture.json("elsewhere", "trail_projects", Value::Null);

    assert_eq!(projects[0]["name"], "Demo");
    assert_eq!(projects[0]["update_count"], 0);
}

#[test]
fn project_argument_overrides_the_folder() {
    let fixture = Fixture::new();

    let status = fixture.json("elsewhere", "trail_status", json!({ "project": "demo" }));

    assert_eq!(status["project"]["name"], "Demo");
}

#[test]
fn trail_errors_are_tool_errors_with_agent_hints() {
    let fixture = Fixture::new();

    let (text, is_error) = fixture.call("elsewhere", "trail_status", json!({}));
    assert!(is_error);
    assert!(
        text.contains("Ask the user whether to run `trail init`"),
        "{text}"
    );

    let (text, is_error) = fixture.call("demo", "trail_log", json!({ "project": "nope" }));
    assert!(is_error);
    assert!(text.contains("Call trail_projects"), "{text}");

    let (text, is_error) = fixture.call("demo", "trail_add", json!({ "title": " ", "body": "B" }));
    assert!(is_error);
    assert_eq!(text, "title can't be empty");
}

#[test]
fn bad_arguments_are_tool_errors_and_nothing_is_saved() {
    let fixture = Fixture::new();

    for arguments in [
        json!({ "title": "T" }),
        json!({ "title": "T", "body": "B", "colour": "red" }),
        json!({ "title": 5, "body": "B" }),
    ] {
        let (text, is_error) = fixture.call("demo", "trail_add", arguments);
        assert!(is_error);
        assert!(text.starts_with("Invalid arguments: "), "{text}");
    }
    let status = fixture.json("demo", "trail_status", json!({}));
    assert!(status["latest_update"].is_null());
}

#[test]
fn unknown_tools_return_none() {
    let fixture = Fixture::new();

    assert!(call(&fixture.conn, &fixture.root, "trail_delete", &json!({})).is_none());
}
