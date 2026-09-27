use chrono::{DateTime, Utc};

pub struct UpdateDraft {
    pub title: Option<String>,
    pub project_id: Option<String>,
    pub body: Option<String>,
    pub next: Option<String>,
    pub branch: Option<String>,
    pub commit_sha: Option<String>,
}

pub struct Update {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub body: String,
    pub next: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// The git branch and `HEAD` commit when the update was written. `None` for
    /// non-git projects, updates from before v0.7.1, and detached `HEAD`/no commits.
    pub branch: Option<String>,
    pub commit_sha: Option<String>,
}

#[derive(PartialEq)]
pub enum UpdateStep {
    Title,
    Body,
    Next,
    Confirm,
}
