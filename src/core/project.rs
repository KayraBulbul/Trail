use std::fmt;
use std::{io, path::Path};

use chrono::{DateTime, Utc};
use rusqlite::Connection;
use serde::Serialize;

use crate::core::status::ProjectInfo;
use crate::git::repo;
use crate::types::project::ProjectDraft;
use crate::{database::sqlite, types::project::Project};

#[derive(Serialize)]
pub struct ProjectSummary {
    pub name: String,
    pub directory: String,
    pub is_git: bool,
    pub update_count: usize,
    pub last_update_at: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
pub struct Init {
    pub project: ProjectInfo,
    pub inside: Option<String>, // name of the enclosing project, if any
}

#[derive(Debug)]
pub enum CoreError {
    NoProjects,
    NameNotFound,
    AlreadyProject(String),
    Invalid(String),
    Io(io::Error),
    Failed(String),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreError::NoProjects => write!(f, "no project covers this folder"),
            CoreError::NameNotFound => write!(f, "no project has that name"),
            CoreError::AlreadyProject(name) => write!(f, "{name} is already a project"),
            CoreError::Invalid(err) => write!(f, "{err}"),
            CoreError::Io(error) => write!(f, "couldn't read this folder: {error}"),
            CoreError::Failed(stderr) => write!(f, "database failed: {stderr}"),
        }
    }
}

impl From<rusqlite::Error> for CoreError {
    fn from(error: rusqlite::Error) -> Self {
        CoreError::Failed(error.to_string())
    }
}

impl From<std::io::Error> for CoreError {
    fn from(error: std::io::Error) -> Self {
        CoreError::Io(error)
    }
}

pub fn resolve_project(
    conn: &Connection,
    dir: &Path,
    name: Option<&str>,
) -> Result<Project, CoreError> {
    let projects = sqlite::get_projects(conn)?;
    if let Some(name) = name {
        let Some(project) = projects
            .into_iter()
            .find(|proj| proj.name.eq_ignore_ascii_case(name))
        else {
            return Err(CoreError::NameNotFound);
        };
        return Ok(project);
    }

    let normalised_dir = std::fs::canonicalize(dir)?;
    projects
        .into_iter()
        // Older TUI versions saved directories as typed (`.`, `~/...`). Those can't be
        // resolved reliably: `.` would match whatever folder Trail runs from.
        .filter(|project| Path::new(&project.directory).is_absolute())
        .filter_map(|project| {
            let project_dir = std::fs::canonicalize(&project.directory).ok()?;
            Some((project_dir, project))
        })
        .filter(|(project_dir, _)| normalised_dir.starts_with(project_dir))
        .max_by_key(|(project_dir, _)| project_dir.components().count())
        .map(|(_, project)| project)
        .ok_or(CoreError::NoProjects)
}

pub fn init_project(conn: &Connection, dir: &Path, name: Option<&str>) -> Result<Init, CoreError> {
    let dir = std::fs::canonicalize(dir)?;

    let inside = match resolve_project(conn, &dir, None) {
        Ok(project) => {
            if std::fs::canonicalize(&project.directory)? == dir {
                return Err(CoreError::AlreadyProject(project.name));
            }
            Some(project.name)
        }
        Err(CoreError::NoProjects) => None,
        Err(e) => return Err(e),
    };

    let name = match name {
        Some(name) => name.to_string(),
        None => dir
            .file_name()
            .ok_or(CoreError::Invalid(
                "couldn't get a name from this folder, pass one with --name".into(),
            ))?
            .to_string_lossy()
            .into_owned(),
    };

    if name.trim().is_empty() {
        return Err(CoreError::Invalid("project name can't be empty".into()));
    }

    if sqlite::get_projects(conn)?
        .iter()
        .any(|project| name.eq_ignore_ascii_case(project.name.as_str()))
    {
        return Err(CoreError::Invalid(format!(
            "a project named {name} already exists, pick another with --name"
        )));
    }

    let directory = dir.to_string_lossy().into_owned();
    sqlite::insert_project(
        conn,
        &ProjectDraft {
            name: Some(name.clone()),
            directory: Some(directory.clone()),
        },
    )?;

    let project = ProjectInfo { name, directory };

    Ok(Init { project, inside })
}

/// Every project, most recently updated first, with its update count and git state.
pub fn list_projects(conn: &Connection) -> Result<Vec<ProjectSummary>, CoreError> {
    sqlite::get_projects(conn)?
        .into_iter()
        .map(|project| {
            let updates = sqlite::get_updates(conn, &project.id)?;
            Ok(ProjectSummary {
                is_git: repo::detect(Path::new(&project.directory)).is_some(),
                update_count: updates.len(),
                last_update_at: updates.first().map(|update| update.updated_at),
                name: project.name,
                directory: project.directory,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests;
