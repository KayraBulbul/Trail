//! Human-readable output for each command. `--json` output is serde's, not this.

use std::path::Path;

use crate::{
    core::{
        project::{Init, ProjectSummary},
        status::{GitStatus, Status, UpdateInfo},
    },
    format,
};

const NO_UPDATES: &str = "No updates yet. Add one with: trail add --title ... --body ...";

pub fn status(status: &Status) -> String {
    let name = &status.project.name;
    let width = name.chars().count().max("Git".len());
    let mut lines = vec![format!(
        "{name:<width$}  {}",
        tilde(&status.project.directory)
    )];
    if let Some(git) = &status.git {
        lines.push(format!("{:<width$}  {}", "Git", git_line(git)));
    }
    lines.push(String::new());

    match &status.latest_update {
        Some(update) => {
            lines.push(format!("Latest update  ({})", update_meta(update)));
            lines.push(indent(&update.title));
            lines.push(indent(&update.body));
            lines.push("Next".to_string());
            lines.push(indent(&update.next));
        }
        None => lines.push(NO_UPDATES.to_string()),
    }
    lines.join("\n")
}

pub fn log(updates: &[UpdateInfo]) -> String {
    if updates.is_empty() {
        return NO_UPDATES.to_string();
    }
    updates
        .iter()
        .map(|update| {
            format!(
                "{}  ({})\n{}\n{}",
                update.title,
                update_meta(update),
                indent(&update.body),
                indent(&format!("Next: {}", update.next)),
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub fn added(project: &str, update: &UpdateInfo) -> String {
    let written_on = format::written_on(update.branch.as_deref(), update.commit_sha.as_deref())
        .map(|written_on| format!(" (written on {written_on})"))
        .unwrap_or_default();
    format!("Added to {project}: {}{written_on}", update.title)
}

pub fn projects(projects: &[ProjectSummary]) -> String {
    if projects.is_empty() {
        return "No projects yet. Run `trail init` in a project's folder to create one."
            .to_string();
    }
    let name_width = column_width(projects.iter().map(|project| project.name.as_str()));
    let directories: Vec<String> = projects
        .iter()
        .map(|project| tilde(&project.directory))
        .collect();
    let directory_width = column_width(directories.iter().map(String::as_str));
    let updates: Vec<String> = projects
        .iter()
        .map(|project| plural(project.update_count, "update"))
        .collect();
    let updates_width = column_width(updates.iter().map(String::as_str));

    projects
        .iter()
        .zip(directories.iter().zip(&updates))
        .map(|(project, (directory, updates))| {
            let git = if project.is_git { "git" } else { "" };
            let last = project
                .last_update_at
                .map(|time| format!("   last update {}", format::relative_time(time)))
                .unwrap_or_default();
            let line = format!(
                "{:<name_width$}   {directory:<directory_width$}   {git:<3}   {updates:<updates_width$}{last}",
                project.name
            );
            line.trim_end().to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn init(init: &Init) -> String {
    let project = &init.project;
    let mut text = format!(
        "Created project {} at {}",
        project.name,
        tilde(&project.directory)
    );
    if let Some(inside) = &init.inside {
        text.push_str(&format!(
            "\nNote: this folder is inside {inside}. Commands run here will now use {}.",
            project.name
        ));
    }
    text
}

/// `main · 2 files changed · last commit 3h ago`, matching the TUI's summary line.
fn git_line(git: &GitStatus) -> String {
    let mut parts = vec![git.branch.clone().unwrap_or("detached HEAD".to_string())];
    if git.files_changed > 0 {
        parts.push(format!("{} changed", plural(git.files_changed, "file")));
    }
    if let Some(last_commit) = git.last_commit {
        parts.push(format!(
            "last commit {}",
            format::relative_time(last_commit)
        ));
    }
    parts.join(" · ")
}

/// `2d ago · written on main @ 94bd07e · 4 commits since`. No commits since is left out.
fn update_meta(update: &UpdateInfo) -> String {
    let mut parts = vec![format::relative_time(update.created_at)];
    if let Some(written_on) =
        format::written_on(update.branch.as_deref(), update.commit_sha.as_deref())
    {
        parts.push(format!("written on {written_on}"));
    }
    if let Some(count) = update.commits_since.filter(|&count| count > 0) {
        parts.push(format!("{} since", plural(count, "commit")));
    }
    parts.join(" · ")
}

/// Indents each line by two spaces, leaving blank lines empty (no trailing spaces).
fn indent(text: &str) -> String {
    text.lines()
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("  {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn plural(count: usize, noun: &str) -> String {
    if count == 1 {
        format!("1 {noun}")
    } else {
        format!("{count} {noun}s")
    }
}

fn column_width<'a>(values: impl Iterator<Item = &'a str>) -> usize {
    values.map(|value| value.chars().count()).max().unwrap_or(0)
}

/// Shortens paths under the home folder to `~/...`.
fn tilde(directory: &str) -> String {
    let path = Path::new(directory);
    match dirs::home_dir().and_then(|home| path.strip_prefix(home).ok().map(Path::to_path_buf)) {
        Some(rest) if rest.as_os_str().is_empty() => "~".to_string(),
        Some(rest) => format!("~/{}", rest.display()),
        None => directory.to_string(),
    }
}
