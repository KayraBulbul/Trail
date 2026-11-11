use super::Input;
use crate::{database::sqlite, types::plan::PlanItem};
use rusqlite::{Connection, Result};

impl Input {
    pub fn edit_plan_item(&mut self, item: PlanItem) {
        self.reset_all();
        self.input = item.text.clone();
        self.character_index = self.input.chars().count();
        self.editing_plan_item = Some(item);
    }

    /// Saves the typed text as a new plan item, or as the edited item's text.
    /// The caller checks it isn't blank. Returns the item's id.
    pub fn submit_plan_item(&mut self, conn: &Connection, project_id: &str) -> Result<i64> {
        let text = self.input.trim();
        let id = match &self.editing_plan_item {
            Some(item) => {
                sqlite::edit_plan_item(conn, item.id, text)?;
                item.id
            }
            None => sqlite::insert_plan_item(conn, project_id, text)?,
        };

        self.reset_all();
        Ok(id)
    }
}
