use crate::{
    database::sqlite,
    types::{project::Project, update::Update},
    ui::input::Input,
    update::updater::Release,
};

use crossterm::event::Event;
use ratatui::{
    DefaultTerminal,
    widgets::{ListState, TableState},
};
use rusqlite::{Connection, Result};
use std::{io, sync::mpsc::Receiver, time::Duration};

mod keybindings;
mod render;

#[cfg(test)]
mod tests;

#[derive(PartialEq)]
pub enum BrowserPane {
    Projects,
    LatestUpdate,
}

pub struct App {
    pub update_rx: Receiver<Release>,
    pub pending_release: Option<Release>,
    pub show_project_input: bool,
    pub show_update_input: bool,
    pub show_update_table: bool,
    pub show_update_popup: bool,
    pub show_help: bool,
    pub help_scroll: u16,
    pub detail_scroll: u16,
    pub confirmation_scroll: u16,
    pub projects: Vec<Project>,
    pub updates: Vec<Update>,
    pub update_selection: TableState,
    pub project_selection: ListState,
    pub opened_project_id: Option<String>,
    pub opened_update_id: Option<String>,
    pub pending_project_delete_id: Option<String>,
    pub pending_update_delete_id: Option<String>,
    pub focused_pane: BrowserPane,
    pub err: Option<String>,
    pub exit: bool,
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal, conn: &Connection) -> io::Result<()> {
        let mut text_in = Input::new();

        if let Err(err) = self.reload_projects(conn) {
            self.err = Some(format!("Couldn't load projects: {err}"));
        }

        while !self.exit {
            if let Ok(release) = self.update_rx.try_recv() {
                self.pending_release = Some(release);
            }
            if self.pending_release.is_some() && self.is_idle() {
                self.show_update_popup = true;
            }

            terminal.draw(|frame| self.render(frame, &mut text_in))?;

            if crossterm::event::poll(Duration::from_millis(250))? {
                if let Event::Key(key_event) = crossterm::event::read()? {
                    self.handle_key_event(key_event, &mut text_in, conn)?;
                }
            }
        }

        Ok(())
    }

    fn is_idle(&self) -> bool {
        !self.show_update_table
            && !self.show_update_input
            && !self.show_project_input
            && !self.show_help
    }

    fn start_update_edit(&mut self, conn: &Connection, text_in: &mut Input, id: &str) {
        match sqlite::get_update(conn, id) {
            Ok(update) => {
                text_in.edit_update(update);
                self.show_update_input = true;
                self.err = None;
            }
            Err(error) => self.err = Some(format!("Couldn't load update: {error}")),
        }
    }

    fn displayed_update(&self) -> Option<&Update> {
        self.updates
            .iter()
            .find(|update| {
                self.opened_project_id.as_deref() == Some(update.project_id.as_str())
                    && self.opened_update_id.as_deref() == Some(update.id.as_str())
            })
            .or_else(|| {
                self.updates.iter().find(|update| {
                    self.opened_project_id.as_deref() == Some(update.project_id.as_str())
                })
            })
    }

    fn delete_project(&mut self, conn: &Connection, id: &str) {
        if let Err(error) = sqlite::delete_project(conn, id) {
            self.err = Some(format!("Unable to delete project: {error}"));
            return;
        }

        self.err = None;
        if self.opened_project_id.as_deref() == Some(id) {
            self.opened_project_id = None;
            self.clear_updates();
        }
        let previous_index = self.project_selection.selected().unwrap_or(0);
        if let Err(error) = self.reload_projects(conn) {
            self.err = Some(format!("Unable to reload projects: {error}"));
        } else {
            let selection = if self.projects.is_empty() {
                None
            } else {
                Some(previous_index.min(self.projects.len() - 1))
            };
            self.project_selection.select(selection);
        }
    }

    fn delete_update(&mut self, conn: &Connection, id: &str) {
        if let Err(error) = sqlite::delete_update(conn, id) {
            self.err = Some(format!("Couldn't delete update: {error}"));
            return;
        }

        self.err = None;
        if self.opened_update_id.as_deref() == Some(id) {
            self.opened_update_id = None;
        }
        let previous_index = self.update_selection.selected().unwrap_or(0);
        if let Err(error) = self.reload_updates(conn) {
            self.err = Some(format!("Couldn't reload updates: {error}"));
        } else if self.updates.is_empty() {
            self.update_selection = TableState::default();
        } else {
            self.update_selection
                .select(Some(previous_index.min(self.updates.len() - 1)));
        }
    }

    fn clear_updates(&mut self) {
        self.detail_scroll = 0;
        self.updates.clear();
        self.update_selection = TableState::default();
        self.opened_update_id = None;
    }

    fn reload_projects(&mut self, conn: &Connection) -> Result<()> {
        let projects = sqlite::get_projects(conn)?;

        let selected = if projects.is_empty() { None } else { Some(0) };

        self.projects = projects;
        self.project_selection.select(selected);

        Ok(())
    }

    fn reload_updates(&mut self, conn: &Connection) -> Result<()> {
        let Some(id) = self.opened_project_id.clone() else {
            unreachable!();
        };

        let updates = sqlite::get_updates(conn, id.as_str())?;

        let selected = if updates.is_empty() { None } else { Some(0) };

        self.detail_scroll = 0;
        self.updates = updates;
        self.update_selection.select(selected);

        Ok(())
    }
}
