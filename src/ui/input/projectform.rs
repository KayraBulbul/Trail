use super::Input;
use crate::{database::sqlite, types::project::ProjectStep};
use rusqlite::{Connection, Result};
use std::path::{Path, PathBuf};
use std::{env, fs, io};

impl Input {
    pub(super) fn reset_project(&mut self) {
        self.project.name = None;
        self.project.directory = None;
    }

    pub fn submit_name(&mut self) {
        if self.input.trim().chars().count() > 0 {
            self.project.name = Some(self.input.clone());
        }

        self.input.clear();
        self.reset_cursor();
    }

    pub fn submit_directory(&mut self) {
        if self.input.trim().chars().count() > 0 {
            self.project.directory = Some(self.input.clone());
        }
        if self.project.name.is_none() {
            self.project.name = Path::new(&self.input)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned());
        }

        self.input.clear();
        self.reset_cursor();
    }

    pub fn submit_project(&mut self, conn: &Connection) -> Result<String> {
        let id = sqlite::insert_project(conn, &self.project)?;

        self.project_step = ProjectStep::Name;
        self.reset_project();

        Ok(id)
    }

    pub fn validate_path(&self) -> io::Result<()> {
        let directory = self.project.directory.as_deref().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "Please enter a directory.")
        })?;

        let path = if directory == "~" || directory.starts_with("~/") {
            let home = env::home_dir().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "Could not determine your home directory.",
                )
            })?;

            if directory == "~" {
                home
            } else {
                home.join(&directory[2..])
            }
        } else {
            PathBuf::from(directory)
        };

        let abs_path = fs::canonicalize(path)?;
        let attr = fs::metadata(&abs_path)?;

        if !attr.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "The selected path is not a directory.",
            ));
        }

        Ok(())
    }
}
