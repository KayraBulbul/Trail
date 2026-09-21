use super::Input;
use crate::{database::sqlite, types::update::UpdateStep};
use rusqlite::{Connection, Result};

impl Input {
    pub(super) fn reset_update(&mut self) {
        self.update.title = None;
        self.update.project_id = None;
        self.update.body = None;
        self.update.next = None;
    }

    pub fn submit_title(&mut self) {
        if self.input.trim().chars().count() > 0 {
            self.update.title = Some(self.input.clone());
        }

        self.input.clear();
        self.reset_cursor();
    }

    pub fn submit_body(&mut self) {
        if self.input.trim().chars().count() > 0 {
            self.update.body = Some(self.input.clone());
        }

        self.input.clear();
        self.reset_cursor();
    }

    pub fn submit_next(&mut self) {
        if self.input.trim().chars().count() > 0 {
            self.update.next = Some(self.input.clone());
        }

        if self.update.next.is_none() {
            self.update.next = Some(String::from("None"));
        }

        self.input.clear();
        self.reset_cursor();
    }

    pub fn submit_update(&mut self, conn: &Connection, project_id: String) -> Result<String> {
        self.update.project_id = Some(project_id.clone());
        let id = sqlite::insert_update(conn, &self.update)?;

        self.update_step = UpdateStep::Title;
        self.reset_update();

        Ok(id)
    }
}
