use std::collections::HashSet;

use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde::Serialize;

use crate::{
    core::project::CoreError,
    database::sqlite,
    types::{
        plan::{PlanItem, PlanState},
        project::Project,
    },
};

#[derive(Serialize)]
pub struct PlanItemInfo {
    pub id: i64,
    pub text: String,
    pub state: PlanState,
    pub created_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub closed_by_update: Option<String>,
}

impl From<PlanItem> for PlanItemInfo {
    fn from(item: PlanItem) -> Self {
        PlanItemInfo {
            id: item.id,
            text: item.text,
            state: item.state,
            created_at: item.created_at,
            closed_at: item.closed_at,
            closed_by_update: item.closed_by_update,
        }
    }
}

/// Changes to a project's plan, applied together or not at all.
#[derive(Default)]
pub struct PlanEdit<'a> {
    pub add: &'a [String],
    pub done: &'a [i64],
    pub drop: &'a [i64],
}

/// The project's open plan items in plan order, or every item with `all`.
pub fn plan(
    conn: &Connection,
    project: &Project,
    all: bool,
) -> Result<Vec<PlanItemInfo>, CoreError> {
    Ok(sqlite::get_plan_items(conn, &project.id)?
        .into_iter()
        .filter(|item| all || item.state == PlanState::Open)
        .map(PlanItemInfo::from)
        .collect())
}

/// Adds, finishes and drops plan items. Everything is checked first, so one bad
/// id or empty item changes nothing. Returns the added items, then the closed ones.
pub fn edit_plan(
    conn: &Connection,
    project: &Project,
    edit: &PlanEdit,
) -> Result<Vec<PlanItemInfo>, CoreError> {
    if edit.add.is_empty() && edit.done.is_empty() && edit.drop.is_empty() {
        return Err(CoreError::Invalid(
            "nothing to change: give items to add, done or drop".into(),
        ));
    }
    let texts = edit
        .add
        .iter()
        .map(|text| item_text(text))
        .collect::<Result<Vec<_>, _>>()?;
    let ids: Vec<i64> = edit.done.iter().chain(edit.drop).copied().collect();

    let tx = sqlite::write_transaction(conn)?;
    check_open_items(&tx, project, &ids)?;
    let mut changed = Vec::new();
    for text in texts {
        changed.push(sqlite::insert_plan_item(&tx, &project.id, text)?);
    }
    close_items(&tx, edit.done, PlanState::Done, None)?;
    close_items(&tx, edit.drop, PlanState::Dropped, None)?;
    changed.extend(&ids);
    let changed = changed
        .into_iter()
        .map(|id| fetch(&tx, id))
        .collect::<Result<Vec<_>, _>>()?;
    tx.commit()?;
    Ok(changed)
}

/// Fails unless every id is an open item in `project`, listed once. Call it inside
/// a `write_transaction`, so the items can't close before they're written.
pub(super) fn check_open_items(
    conn: &Connection,
    project: &Project,
    ids: &[i64],
) -> Result<(), CoreError> {
    let mut seen = HashSet::new();
    for &id in ids {
        if !seen.insert(id) {
            return Err(CoreError::Invalid(format!(
                "plan item {id} is listed twice"
            )));
        }
        let Some(item) = sqlite::get_plan_item(conn, id)? else {
            return Err(CoreError::Invalid(format!("there's no plan item {id}")));
        };
        if item.project_id != project.id {
            return Err(CoreError::Invalid(format!(
                "plan item {id} isn't in {}'s plan",
                project.name
            )));
        }
        if item.state != PlanState::Open {
            return Err(CoreError::Invalid(format!(
                "plan item {id} is already {}",
                item.state.as_str()
            )));
        }
    }
    Ok(())
}

/// Closes items already checked with `check_open_items`.
pub(super) fn close_items(
    conn: &Connection,
    ids: &[i64],
    state: PlanState,
    update_id: Option<&str>,
) -> Result<(), CoreError> {
    for &id in ids {
        sqlite::close_plan_item(conn, id, state, update_id)?;
    }
    Ok(())
}

/// An item's text, trimmed. Items are one line, so a newline is a mistake.
fn item_text(text: &str) -> Result<&str, CoreError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(CoreError::Invalid("plan items can't be empty".into()));
    }
    if text.contains('\n') {
        return Err(CoreError::Invalid(
            "plan items are one line; add each line as its own item".into(),
        ));
    }
    Ok(text)
}

fn fetch(conn: &Connection, id: i64) -> Result<PlanItemInfo, CoreError> {
    sqlite::get_plan_item(conn, id)?
        .map(PlanItemInfo::from)
        .ok_or_else(|| CoreError::Failed(format!("plan item {id} disappeared")))
}

#[cfg(test)]
mod tests;
