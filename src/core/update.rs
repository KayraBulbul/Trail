use std::path::Path;

use rusqlite::Connection;

use crate::{
    core::{
        plan::{check_open_items, close_items},
        project::CoreError,
        status::{UpdateInfo, update_info},
    },
    database::sqlite,
    git::repo::{self, Head},
    types::{plan::PlanState, project::Project, update::UpdateDraft},
};

pub fn git_context(dir: &Path) -> (Option<String>, Option<String>) {
    // (branch, commit_sha)
    (
        repo::detect(dir).and_then(|info| match info.head {
            Head::Branch(name) => Some(name),
            Head::Detached(_) => None,
        }),
        repo::head_commit(dir),
    )
}

pub fn add_update(
    conn: &Connection,
    project: &Project,
    title: &str,
    body: &str,
    next: Option<&str>,
    done: &[i64],
) -> Result<UpdateInfo, CoreError> {
    if title.trim().is_empty() {
        return Err(CoreError::Invalid("title can't be empty".into()));
    }

    if body.trim().is_empty() {
        return Err(CoreError::Invalid("body can't be empty".into()));
    }

    // Check the plan items first, so a bad id doesn't leave a half-written update.
    check_open_items(conn, project, done)?;

    let next = next.unwrap_or("None");

    let (branch, commit_sha) = git_context(Path::new(&project.directory));

    let draft = UpdateDraft {
        title: Some(title.to_string()),
        project_id: Some(project.id.clone()),
        body: Some(body.to_string()),
        next: Some(next.to_string()),
        branch,
        commit_sha,
    };

    let tx = conn.unchecked_transaction()?;
    let id = sqlite::insert_update(&tx, &draft)?;
    close_items(&tx, done, PlanState::Done, Some(&id))?;
    tx.commit()?;
    let update = sqlite::get_update(conn, &id)?;
    let info = update_info(update, Path::new(&project.directory));
    Ok(info)
}

#[cfg(test)]
mod tests;
