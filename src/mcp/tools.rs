//! Trail's MCP tools: their definitions for `tools/list` and what `tools/call` runs.
//! Each tool wraps the same `core` function as the matching CLI command.

use std::path::Path;

use rusqlite::Connection;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};

use crate::{
    core::{
        plan::{PlanEdit, edit_plan, plan},
        project::{CoreError, list_projects, resolve_project},
        status,
        update::add_update,
    },
    types::project::Project,
};

const PROJECT_ARGUMENT: &str =
    "Project name. Defaults to the project containing the folder the server runs in.";

/// The `tools` array for a `tools/list` result, in a fixed order.
pub fn definitions() -> Value {
    json!([
        {
            "name": "trail_status",
            "title": "Where work left off",
            "description": "The project's latest update (what was done and what to do next), its open plan items (the work still to do), and its git state. Call this at the start of a session.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": { "type": "string", "description": PROJECT_ARGUMENT }
                },
                "additionalProperties": false
            }
        },
        {
            "name": "trail_log",
            "title": "Recent updates",
            "description": "The project's most recent updates, newest first.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": { "type": "string", "description": PROJECT_ARGUMENT },
                    "count": { "type": "integer", "minimum": 1, "default": 5, "description": "How many updates to return." }
                },
                "additionalProperties": false
            }
        },
        {
            "name": "trail_add",
            "title": "Record an update",
            "description": "Record what you did this session and what to do next. Call this at the end of a session. The current git branch and commit are saved with it. Updates can't be edited or deleted afterwards. Pass `done` to mark the plan items this session finished.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": { "type": "string", "description": PROJECT_ARGUMENT },
                    "title": { "type": "string", "description": "A short title for the update." },
                    "body": { "type": "string", "description": "What was done." },
                    "next": { "type": "string", "description": "The immediate next step. Work that lasts beyond it belongs in the plan." },
                    "done": { "type": "array", "items": { "type": "integer" }, "description": "Ids of open plan items this session finished." }
                },
                "required": ["title", "body"],
                "additionalProperties": false
            }
        },
        {
            "name": "trail_plan",
            "title": "The plan",
            "description": "The project's plan: the work still to do, in order. Open items only unless `all` is true.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": { "type": "string", "description": PROJECT_ARGUMENT },
                    "all": { "type": "boolean", "default": false, "description": "Include done and dropped items." }
                },
                "additionalProperties": false
            }
        },
        {
            "name": "trail_plan_edit",
            "title": "Change the plan",
            "description": "Add, finish or drop plan items, all at once or not at all. Items are pieces of work that take one or more sessions, not small TODOs: add one only when the user agrees to the work or it's left over from this session. Explain drops in your next update. To finish items while recording an update, pass `done` to trail_add instead.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project": { "type": "string", "description": PROJECT_ARGUMENT },
                    "add": { "type": "array", "items": { "type": "string" }, "description": "New items, one line each, added to the end of the plan." },
                    "done": { "type": "array", "items": { "type": "integer" }, "description": "Ids of open items that are finished." },
                    "drop": { "type": "array", "items": { "type": "integer" }, "description": "Ids of open items that won't be done." }
                },
                "additionalProperties": false
            }
        },
        {
            "name": "trail_projects",
            "title": "List projects",
            "description": "Every Trail project with its folder, whether it's a git repo, its update count, and how many plan items are open.",
            "inputSchema": { "type": "object", "additionalProperties": false }
        }
    ])
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StatusArguments {
    project: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LogArguments {
    project: Option<String>,
    count: Option<usize>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AddArguments {
    project: Option<String>,
    title: String,
    body: String,
    next: Option<String>,
    #[serde(default)]
    done: Vec<i64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanArguments {
    project: Option<String>,
    #[serde(default)]
    all: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PlanEditArguments {
    project: Option<String>,
    #[serde(default)]
    add: Vec<String>,
    #[serde(default)]
    done: Vec<i64>,
    #[serde(default)]
    drop: Vec<i64>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct NoArguments {}

/// Runs a tool and returns its `tools/call` result, or `None` if there's no tool
/// with that name. Bad arguments and Trail errors come back as results with
/// `isError: true`, so the agent can read the message and try again.
pub fn call(conn: &Connection, cwd: &Path, name: &str, arguments: &Value) -> Option<Value> {
    let result = match name {
        "trail_status" => run(arguments, |args: StatusArguments| {
            let project = find(conn, cwd, args.project)?;
            Ok(to_json(&status::status(conn, &project)?))
        }),
        "trail_log" => run(arguments, |args: LogArguments| {
            let project = find(conn, cwd, args.project)?;
            Ok(to_json(&status::log(
                conn,
                &project,
                args.count.unwrap_or(5),
            )?))
        }),
        "trail_add" => run(arguments, |args: AddArguments| {
            let project = find(conn, cwd, args.project)?;
            let update = add_update(
                conn,
                &project,
                &args.title,
                &args.body,
                args.next.as_deref(),
                &args.done,
            )?;
            Ok(to_json(&update))
        }),
        "trail_plan" => run(arguments, |args: PlanArguments| {
            let project = find(conn, cwd, args.project)?;
            Ok(to_json(&plan(conn, &project, args.all)?))
        }),
        "trail_plan_edit" => run(arguments, |args: PlanEditArguments| {
            let project = find(conn, cwd, args.project)?;
            let edit = PlanEdit {
                add: &args.add,
                done: &args.done,
                drop: &args.drop,
            };
            Ok(to_json(&edit_plan(conn, &project, &edit)?))
        }),
        "trail_projects" => run(arguments, |_: NoArguments| {
            Ok(to_json(&list_projects(conn)?))
        }),
        _ => return None,
    };

    let (text, is_error) = match result {
        Ok(text) => (text, false),
        Err(message) => (message, true),
    };
    Some(json!({
        "content": [{ "type": "text", "text": text }],
        "isError": is_error,
    }))
}

/// Parses the arguments into `A`, then runs the tool. A missing `arguments` counts as `{}`.
fn run<A: DeserializeOwned>(
    arguments: &Value,
    tool: impl FnOnce(A) -> Result<String, CoreError>,
) -> Result<String, String> {
    let arguments = if arguments.is_null() {
        json!({})
    } else {
        arguments.clone()
    };
    let arguments = serde_json::from_value(arguments)
        .map_err(|error| format!("Invalid arguments: {error}."))?;
    tool(arguments).map_err(|error| error_message(&error))
}

fn find(conn: &Connection, cwd: &Path, project: Option<String>) -> Result<Project, CoreError> {
    resolve_project(conn, cwd, project.as_deref())
}

fn to_json(value: &impl Serialize) -> String {
    serde_json::to_string_pretty(value).expect("output types always serialize")
}

/// Like the CLI's error messages, but the hints point at tools and arguments.
fn error_message(error: &CoreError) -> String {
    match error {
        CoreError::NoProjects => format!(
            "{error}. Ask the user whether to run `trail init` in their project folder, or pass the `project` argument."
        ),
        CoreError::NameNotFound => format!("{error}. Call trail_projects to see the names."),
        _ => error.to_string(),
    }
}

#[cfg(test)]
mod tests;
