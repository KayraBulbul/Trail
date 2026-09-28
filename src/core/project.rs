use std::fmt;
use std::{io, path::Path};

use rusqlite::Connection;

use crate::{database::sqlite, types::project::Project};

#[derive(Debug)]
pub enum CoreError {
    NoProjects,
    NameNotFound,
    Io(io::Error),
    Failed(String),
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CoreError::NoProjects => write!(f, "no project covers this folder"),
            CoreError::NameNotFound => write!(f, "no project has that name"),
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
        .filter_map(|project| {
            let project_dir = std::fs::canonicalize(&project.directory).ok()?;
            Some((project_dir, project))
        })
        .filter(|(project_dir, _)| normalised_dir.starts_with(project_dir))
        .max_by_key(|(project_dir, _)| project_dir.components().count())
        .map(|(_, project)| project)
        .ok_or(CoreError::NoProjects)
}

#[cfg(test)]
mod tests;
