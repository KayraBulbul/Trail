use std::{
    env,
    io::{self, Read},
    path::Path,
    process::ExitCode,
};

use clap::{Args, Parser, Subcommand};
use rusqlite::Connection;
use serde::Serialize;

use crate::{
    core::{
        project::{CoreError, init_project, list_projects, resolve_project},
        status,
        update::add_update,
    },
    database::sqlite,
    mcp, update,
};

mod output;

/// Trail allows devs to organise their projects and leave themselves notes/updates so they can
/// pick up where they left off. Run `trail` with no command to open the TUI.
#[derive(Parser)]
#[command(name = "trail", disable_version_flag = true)]
pub struct Cli {
    /// Print version information.
    #[arg(short, long)]
    pub version: bool,

    /// Print JSON instead of text, for agents and scripts.
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Show this folder's project, its latest update, and git info.
    Status {
        #[command(flatten)]
        target: Target,
    },
    /// Show the most recent updates, newest first.
    Log {
        #[command(flatten)]
        target: Target,
        /// How many updates to show.
        #[arg(short = 'n', long, default_value_t = 5)]
        count: usize,
    },
    /// Write a new update. Records the current git branch and commit.
    Add {
        #[command(flatten)]
        target: Target,
        /// A short title for the update.
        #[arg(long)]
        title: String,
        /// What you worked on. Pass `-` to read it from stdin.
        #[arg(long)]
        body: String,
        /// What to do next. Defaults to "None".
        #[arg(long)]
        next: Option<String>,
    },
    /// List all projects.
    Projects,
    /// Register the current folder as a project.
    Init {
        /// The project name. Defaults to the folder name.
        #[arg(long)]
        name: Option<String>,
    },
    /// Print instructions to paste into an AGENTS.md or CLAUDE.md.
    Agents,
    /// Run an MCP server on stdin/stdout, for AI agents that support MCP.
    Mcp,
    /// Install the latest version.
    Update,
}

/// Which project a command applies to.
#[derive(Args)]
pub struct Target {
    /// Use the project with this name instead of the one for the current folder.
    #[arg(short, long)]
    pub project: Option<String>,
}

pub fn print_version() {
    println!("Trail Version: {}", env!("CARGO_PKG_VERSION"));
    if let Ok(Some(latest)) = update::updater::cached_update() {
        println!("Update available: v{latest}. Run `trail update` to install it.");
    }
}

/// What `trail agents` prints: instructions for an AGENTS.md or CLAUDE.md.
pub const AGENTS_SNIPPET: &str = include_str!("agents.md");

pub fn run(command: Command, json: bool) -> Result<ExitCode, Box<dyn std::error::Error>> {
    match command {
        Command::Update => {
            match update::updater::check_update()
                .map_err(|error| format!("Couldn't check for updates: {error}"))?
            {
                Some(release) => update::updater::install_update(release)?,
                None => println!("Trail is up to date (v{}).", env!("CARGO_PKG_VERSION")),
            }
            Ok(ExitCode::SUCCESS)
        }
        Command::Agents => {
            print!("{AGENTS_SNIPPET}");
            Ok(ExitCode::SUCCESS)
        }
        Command::Mcp => {
            let conn = sqlite::create_database()?;
            let cwd = env::current_dir()?;
            mcp::serve(&conn, &cwd, io::stdin().lock(), io::stdout().lock())?;
            Ok(ExitCode::SUCCESS)
        }
        command => {
            let conn = sqlite::create_database()?;
            let cwd = env::current_dir()?;
            match execute(&conn, &cwd, command, json, &mut io::stdin().lock()) {
                Ok(output) => {
                    println!("{output}");
                    if !json && let Ok(Some(latest)) = update::updater::cached_update() {
                        eprintln!("Update available: v{latest}. Run `trail update` to install it.");
                    }
                    Ok(ExitCode::SUCCESS)
                }
                Err(error) => {
                    eprintln!("error: {}", error_message(&error));
                    Ok(ExitCode::FAILURE)
                }
            }
        }
    }
}

/// Runs a command that works on the database and returns what to print.
/// Separate from `run` so tests can pass their own database, folder, and stdin.
fn execute(
    conn: &Connection,
    cwd: &Path,
    command: Command,
    json: bool,
    stdin: &mut impl Read,
) -> Result<String, CoreError> {
    Ok(match command {
        Command::Status { target } => {
            let project = resolve_project(conn, cwd, target.project.as_deref())?;
            let status = status::status(conn, &project)?;
            if json {
                to_json(&status)
            } else {
                output::status(&status)
            }
        }
        Command::Log { target, count } => {
            let project = resolve_project(conn, cwd, target.project.as_deref())?;
            let updates = status::log(conn, &project, count)?;
            if json {
                to_json(&updates)
            } else {
                output::log(&updates)
            }
        }
        Command::Add {
            target,
            title,
            body,
            next,
        } => {
            // Find the project first, so a wrong folder fails before waiting on stdin.
            let project = resolve_project(conn, cwd, target.project.as_deref())?;
            let body = if body == "-" {
                let mut text = String::new();
                stdin.read_to_string(&mut text)?;
                text.trim_end().to_string()
            } else {
                body
            };
            let update = add_update(conn, &project, &title, &body, next.as_deref())?;
            if json {
                to_json(&update)
            } else {
                output::added(&project.name, &update)
            }
        }
        Command::Projects => {
            let projects = list_projects(conn)?;
            if json {
                to_json(&projects)
            } else {
                output::projects(&projects)
            }
        }
        Command::Init { name } => {
            let init = init_project(conn, cwd, name.as_deref())?;
            if json {
                to_json(&init)
            } else {
                output::init(&init)
            }
        }
        Command::Update | Command::Agents | Command::Mcp => {
            unreachable!("handled in run, not a one-shot command")
        }
    })
}

fn to_json(value: &impl Serialize) -> String {
    serde_json::to_string_pretty(value).expect("output types always serialize")
}

/// The error text, plus what to do about it where that's obvious.
fn error_message(error: &CoreError) -> String {
    match error {
        CoreError::NoProjects => {
            format!("{error}. Run `trail init` to create one, or pass --project <name>.")
        }
        CoreError::NameNotFound => format!("{error}. Run `trail projects` to list them."),
        _ => error.to_string(),
    }
}

#[cfg(test)]
mod tests;
