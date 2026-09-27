use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};

use crate::update;

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

pub fn run(command: Command, _json: bool) -> Result<ExitCode, Box<dyn std::error::Error>> {
    let name = match command {
        Command::Update => {
            match update::updater::check_update()
                .map_err(|error| format!("Couldn't check for updates: {error}"))?
            {
                Some(release) => update::updater::install_update(release)?,
                None => println!("Trail is up to date (v{}).", env!("CARGO_PKG_VERSION")),
            }
            return Ok(ExitCode::SUCCESS);
        }
        Command::Status { .. } => "status",
        Command::Log { .. } => "log",
        Command::Add { .. } => "add",
        Command::Projects => "projects",
        Command::Init { .. } => "init",
        Command::Agents => "agents",
    };

    eprintln!("`trail {name}` isn't implemented yet.");
    Ok(ExitCode::FAILURE)
}

#[cfg(test)]
mod tests;
