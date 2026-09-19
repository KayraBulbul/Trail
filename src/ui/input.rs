use std::path::{Path, PathBuf};
use std::{env, fs, io};

use rusqlite::{Connection, Result};

use crate::{
    database::sqlite,
    types::project::{Project, ProjectStep},
};

pub struct Input {
    pub input: String,
    pub character_index: usize,
    pub input_mode: InputMode,
    pub project: Project,
    pub project_step: ProjectStep,
}

#[derive(PartialEq)]
pub enum InputMode {
    Normal,
    Editing,
}

impl Input {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            input_mode: InputMode::Normal,
            character_index: 0,
            project: Project {
                name: None,
                directory: None,
            },
            project_step: ProjectStep::Name,
        }
    }

    pub fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    pub fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }

    pub fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.input.insert(index, new_char);
        self.move_cursor_right();
    }

    fn byte_index(&self) -> usize {
        self.input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.input.len())
    }

    pub fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.character_index != 0;
        if is_not_cursor_leftmost {
            let current_index = self.character_index;
            let from_left_to_current_index = current_index - 1;

            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            let after_char_to_delete = self.input.chars().skip(current_index);

            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.chars().count())
    }

    pub const fn reset_cursor(&mut self) {
        self.character_index = 0;
    }

    fn reset_project(&mut self) {
        self.project.name = None;
        self.project.directory = None;
    }

    pub fn reset_all(&mut self) {
        self.input.clear();
        self.reset_cursor();
        self.reset_project();
        self.project_step = ProjectStep::Name;
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

    pub fn submit_project(&mut self, conn: &Connection) -> Result<()> {
        sqlite::insert_project(conn, &self.project)?;

        self.project_step = ProjectStep::Name;
        self.reset_project();

        Ok(())
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
