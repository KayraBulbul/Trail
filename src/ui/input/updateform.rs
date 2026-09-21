use super::{Input, InputMode};
use crate::{
    database::sqlite,
    types::update::{Update, UpdateDraft, UpdateStep},
};
use rusqlite::{Connection, Result};

impl Input {
    pub fn edit_update(&mut self, update: Update) {
        self.reset_all();
        self.update = UpdateDraft {
            title: Some(update.title.clone()),
            body: Some(update.body.clone()),
            next: Some(update.next.clone()),
            project_id: Some(update.project_id.clone()),
        };
        self.editing_update = Some(update);
        self.input_mode = InputMode::Editing;
        self.prefill_update_field();
    }

    pub fn prefill_update_field(&mut self) {
        self.input = match self.update_step {
            UpdateStep::Title => self.update.title.clone(),
            UpdateStep::Body => self.update.body.clone(),
            UpdateStep::Next => self.update.next.clone(),
            UpdateStep::Confirm => None,
        }
        .unwrap_or_default();
        self.character_index = self.input.chars().count();
    }

    pub(super) fn reset_update(&mut self) {
        self.editing_update = None;
        self.update.title = None;
        self.update.project_id = None;
        self.update.body = None;
        self.update.next = None;
    }

    pub fn submit_title(&mut self) {
        self.update.title = if self.input.trim().is_empty() {
            None
        } else {
            Some(self.input.clone())
        };

        self.input.clear();
        self.reset_cursor();
    }

    pub fn submit_body(&mut self) {
        self.update.body = if self.input.trim().is_empty() {
            None
        } else {
            Some(self.input.clone())
        };

        self.input.clear();
        self.reset_cursor();
    }

    pub fn submit_next(&mut self) {
        self.update.next = Some(if self.input.trim().is_empty() {
            String::from("None")
        } else {
            self.input.clone()
        });

        self.input.clear();
        self.reset_cursor();
    }

    pub fn submit_update(&mut self, conn: &Connection, project_id: String) -> Result<String> {
        self.update.project_id = Some(project_id.clone());
        let id = if let Some(update) = &mut self.editing_update {
            update.title = self.update.title.clone().unwrap();
            update.body = self.update.body.clone().unwrap();
            update.next = self.update.next.clone().unwrap();
            sqlite::edit_update(conn, update)?;
            update.id.clone()
        } else {
            sqlite::insert_update(conn, &self.update)?
        };

        self.update_step = UpdateStep::Title;
        self.reset_update();

        Ok(id)
    }
}
