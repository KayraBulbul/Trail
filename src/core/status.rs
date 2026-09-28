use std::path::Path;

use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde::Serialize;

use crate::{
    core::project::CoreError,
    database::sqlite,
    git::repo::{self, Head, commits_since},
    types::{project::Project, update::Update},
};

#[derive(Serialize)]
pub struct Status {
    pub project: ProjectInfo,
    pub git: Option<GitStatus>,
    pub latest_update: Option<UpdateInfo>,
}

#[derive(Serialize)]
pub struct ProjectInfo {
    pub name: String,
    pub directory: String,
}

#[derive(Serialize)]
pub struct GitStatus {
    pub branch: Option<String>, // None when HEAD is detached
    pub files_changed: usize,
    pub last_commit: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
pub struct UpdateInfo {
    pub id: String,
    pub title: String,
    pub body: String,
    pub next: String,
    pub created_at: DateTime<Utc>,
    pub branch: Option<String>,
    pub commit_sha: Option<String>,
    pub commits_since: Option<usize>,
}

fn update_info(update: Update, dir: &Path) -> UpdateInfo {
    let commits_since = update
        .commit_sha
        .as_deref()
        .and_then(|sha| commits_since(dir, sha));

    UpdateInfo {
        id: update.id,
        title: update.title,
        body: update.body,
        next: update.next,
        created_at: update.created_at,
        branch: update.branch,
        commit_sha: update.commit_sha,
        commits_since,
    }
}

pub fn log(
    conn: &Connection,
    project: &Project,
    count: usize,
) -> Result<Vec<UpdateInfo>, CoreError> {
    Ok(sqlite::get_updates(conn, &project.id)?
        .into_iter()
        .take(count)
        .map(|update| update_info(update, Path::new(&project.directory)))
        .collect())
}

pub fn status(conn: &Connection, project: &Project) -> Result<Status, CoreError> {
    let info = ProjectInfo {
        name: project.name.clone(),
        directory: project.directory.clone(),
    };

    let git = repo::summary(Path::new(&info.directory))
        .ok()
        .map(|sum| GitStatus {
            branch: match sum.head {
                Head::Branch(name) => Some(name),
                Head::Detached(_) => None,
            },
            files_changed: sum.files_changed,
            last_commit: sum.last_commit,
        });

    let latest_update = log(conn, project, 1)?.into_iter().next();

    Ok(Status {
        project: info,
        git,
        latest_update,
    })
}

#[cfg(test)]
mod tests;
