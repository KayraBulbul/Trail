use chrono::{DateTime, Utc};

pub struct ProjectDraft {
    pub name: Option<String>,
    pub directory: Option<String>,
}

pub struct Project {
    pub id: String,
    pub name: String,
    pub directory: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(PartialEq)]
pub enum ProjectStep {
    Name,
    Directory,
    Confirm,
}
