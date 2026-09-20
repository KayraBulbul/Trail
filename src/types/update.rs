use chrono::{DateTime, Utc};

pub struct UpdateDraft {
    pub title: Option<String>,
    pub project_id: Option<String>,
    pub body: Option<String>,
}

pub struct Update {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
