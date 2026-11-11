use chrono::{DateTime, Utc};
use serde::Serialize;

/// Where a plan item stands. Done and dropped items stay in the database, so the
/// plan keeps a record of what was finished or cut.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PlanState {
    Open,
    Done,
    Dropped,
}

impl PlanState {
    pub fn as_str(self) -> &'static str {
        match self {
            PlanState::Open => "open",
            PlanState::Done => "done",
            PlanState::Dropped => "dropped",
        }
    }

    /// Unknown values (from a newer Trail) read as open, so the item still shows.
    pub fn parse(state: &str) -> Self {
        match state {
            "done" => PlanState::Done,
            "dropped" => PlanState::Dropped,
            _ => PlanState::Open,
        }
    }
}

#[derive(Clone)]
pub struct PlanItem {
    /// SQLite's integer row id, short enough to type: `trail plan done 7`.
    pub id: i64,
    pub project_id: String,
    pub text: String,
    /// Sort order within the project. Lower comes first; gaps are fine.
    pub position: i64,
    pub state: PlanState,
    pub created_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    /// The update that closed the item (`trail add --done`), if any.
    pub closed_by_update: Option<String>,
}
