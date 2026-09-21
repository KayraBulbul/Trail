use super::{App, BrowserPane};
use crate::{
    types::{project::ProjectStep, update::UpdateStep},
    ui::input::{self, InputMode},
};
use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};
use rusqlite::Connection;
use std::io;

impl App {
    pub(super) fn handle_key_event(
        &mut self,
        key_event: crossterm::event::KeyEvent,
        text_in: &mut input::Input,
        conn: &Connection,
    ) -> io::Result<()> {
        if key_event.kind == KeyEventKind::Press {
            if self.pending_project_delete_id.is_some() {
                match key_event.code {
                    KeyCode::Enter => {
                        if let Some(id) = self.pending_project_delete_id.take() {
                            self.delete_project(conn, &id);
                        }
                    }
                    KeyCode::Esc => self.pending_project_delete_id = None,
                    _ => {}
                }
                return Ok(());
            }

            if self.pending_update_delete_id.is_some() {
                match key_event.code {
                    KeyCode::Enter => {
                        if let Some(id) = self.pending_update_delete_id.take() {
                            self.delete_update(conn, &id);
                        }
                    }
                    KeyCode::Esc => self.pending_update_delete_id = None,
                    _ => {}
                }
                return Ok(());
            }

            let scroll = if self.show_help {
                Some(&mut self.help_scroll)
            } else if self.show_update_input && text_in.update_step == UpdateStep::Confirm {
                Some(&mut self.confirmation_scroll)
            } else if !self.show_update_input
                && !self.show_project_input
                && !self.show_update_table
                && self.focused_pane == BrowserPane::LatestUpdate
            {
                Some(&mut self.detail_scroll)
            } else {
                None
            };
            if let Some(scroll) = scroll {
                let next = match key_event.code {
                    KeyCode::Down | KeyCode::Char('j') => Some(scroll.saturating_add(1)),
                    KeyCode::Up | KeyCode::Char('k') => Some(scroll.saturating_sub(1)),
                    KeyCode::PageDown => Some(scroll.saturating_add(10)),
                    KeyCode::PageUp => Some(scroll.saturating_sub(10)),
                    KeyCode::Home => Some(0),
                    KeyCode::End => Some(u16::MAX),
                    _ => None,
                };
                if let Some(next) = next {
                    *scroll = next;
                    return Ok(());
                }
            }

            if self.show_project_input && text_in.input_mode == InputMode::Editing {
                match key_event.code {
                    KeyCode::Enter if text_in.project_step == ProjectStep::Name => {
                        text_in.submit_name();
                        if !text_in.project.name.is_none() {
                            self.err = None;
                        }
                        if self.err.is_none() && text_in.project.directory.is_none() {
                            text_in.project_step = ProjectStep::Directory;
                        } else if self.err.is_none() {
                            match text_in.validate_path() {
                                Ok(()) => {
                                    text_in.project_step = ProjectStep::Confirm;
                                }
                                Err(error) => {
                                    self.err = Some(format!("Couldn't validate path: {error}"));
                                    text_in.project_step = ProjectStep::Directory;
                                }
                            }
                        }
                    }
                    KeyCode::Enter if text_in.project_step == ProjectStep::Directory => {
                        text_in.submit_directory();
                        if text_in.project.directory.is_none() {
                            self.err = Some("Directory field needs to be filled.".to_string());
                        } else if text_in.project.name.is_none() {
                            text_in.project_step = ProjectStep::Name;
                            self.err = Some(
                                "Couldn't derive a project name. Please enter one.".to_string(),
                            )
                        } else {
                            match text_in.validate_path() {
                                Ok(()) => {
                                    text_in.project_step = ProjectStep::Confirm;
                                    self.err = None;
                                }
                                Err(error) => {
                                    self.err = Some(format!("Couldn't validate path: {error}"));
                                }
                            }
                        }
                    }
                    KeyCode::Enter if text_in.project_step == ProjectStep::Confirm => {
                        match text_in.submit_project(conn) {
                            Ok(project_id) => {
                                self.err = None;
                                if let Err(err) = self.reload_projects(conn) {
                                    self.err = Some(format!("Couldn't load projects: {err}"));
                                }

                                text_in.reset_all();
                                self.show_project_input = false;

                                let idx = self.projects.iter().position(|p| p.id == project_id);
                                self.project_selection.select(idx);

                                self.opened_project_id = Some(project_id);
                                self.clear_updates();
                                self.focused_pane = BrowserPane::LatestUpdate;
                            }
                            Err(error) => {
                                self.err = Some(format!("{error}. Please try again."));
                                text_in.project_step = ProjectStep::Name;
                            }
                        }
                    }
                    KeyCode::Esc => {
                        text_in.reset_all();
                        self.err = None;
                        text_in.input_mode = InputMode::Normal;
                        self.show_project_input = false;
                    }
                    key if matches!(
                        text_in.project_step,
                        ProjectStep::Name | ProjectStep::Directory
                    ) =>
                    {
                        match key {
                            KeyCode::Char(to_insert) => text_in.enter_char(to_insert),
                            KeyCode::Left => text_in.move_cursor_left(),
                            KeyCode::Right => text_in.move_cursor_right(),
                            KeyCode::Backspace => text_in.delete_char(),
                            _ => {}
                        }
                    }
                    _ => {}
                }
            } else if self.show_update_input && text_in.input_mode == InputMode::Editing {
                match key_event.code {
                    KeyCode::Enter
                        if key_event.modifiers.contains(KeyModifiers::SHIFT)
                            && matches!(
                                text_in.update_step,
                                UpdateStep::Body | UpdateStep::Next
                            ) =>
                    {
                        text_in.enter_char('\n');
                    }
                    KeyCode::Enter if text_in.update_step == UpdateStep::Title => {
                        text_in.submit_title();
                        self.err = None;
                        if text_in.update.title.is_none() {
                            self.err = Some("Must enter an update name.".to_string());
                        } else if self.err.is_none() {
                            text_in.update_step = UpdateStep::Body;
                            text_in.prefill_update_field();
                        }
                    }
                    KeyCode::Enter if text_in.update_step == UpdateStep::Body => {
                        text_in.submit_body();
                        self.err = None;
                        if text_in.update.body.is_none() {
                            self.err = Some("Must enter an update body.".to_string());
                        } else if self.err.is_none() {
                            text_in.update_step = UpdateStep::Next;
                            text_in.prefill_update_field();
                        }
                    }
                    KeyCode::Enter if text_in.update_step == UpdateStep::Next => {
                        text_in.submit_next();
                        text_in.update_step = UpdateStep::Confirm;
                        self.confirmation_scroll = 0;
                    }
                    KeyCode::Enter if text_in.update_step == UpdateStep::Confirm => {
                        let Some(project_id) = self.opened_project_id.clone() else {
                            unreachable!("Update confirmation requires a selected project");
                        };

                        match text_in.submit_update(conn, project_id) {
                            Ok(update_id) => {
                                self.err = None;
                                if let Err(err) = self.reload_updates(conn) {
                                    self.err = Some(format!("Couldn't load updates: {err}"));
                                }

                                text_in.reset_all();
                                self.show_update_input = false;

                                let idx = self.updates.iter().position(|u| u.id == update_id);
                                self.update_selection.select(idx);

                                self.opened_update_id = Some(update_id);
                                self.focused_pane = BrowserPane::LatestUpdate;
                            }
                            Err(err) => {
                                self.err = Some(format!("{err}. Please try again."));
                            }
                        }
                    }
                    KeyCode::Esc => {
                        text_in.reset_all();
                        self.err = None;
                        text_in.input_mode = InputMode::Normal;
                        self.show_update_input = false;
                    }
                    key if matches!(
                        text_in.update_step,
                        UpdateStep::Title | UpdateStep::Body | UpdateStep::Next
                    ) =>
                    {
                        match key {
                            KeyCode::Char(to_insert) => text_in.enter_char(to_insert),
                            KeyCode::Left => text_in.move_cursor_left(),
                            KeyCode::Right => text_in.move_cursor_right(),
                            KeyCode::Backspace => text_in.delete_char(),
                            _ => {}
                        }
                    }
                    _ => {}
                }
            } else if self.show_update_table {
                match key_event.code {
                    KeyCode::Esc => self.show_update_table = false,
                    KeyCode::Char('q') => self.exit = true,
                    KeyCode::Char('j') | KeyCode::Down if !self.updates.is_empty() => {
                        let index = match self.update_selection.selected() {
                            Some(index) => index.saturating_add(1).min(self.updates.len() - 1),
                            None => 0,
                        };
                        self.update_selection.select(Some(index));
                    }
                    KeyCode::Char('k') | KeyCode::Up if !self.updates.is_empty() => {
                        let index = self
                            .update_selection
                            .selected()
                            .unwrap_or(0)
                            .saturating_sub(1);
                        self.update_selection.select(Some(index));
                    }
                    KeyCode::Char('h') | KeyCode::Left if !self.updates.is_empty() => {
                        let column = self
                            .update_selection
                            .selected_column()
                            .unwrap_or(0)
                            .saturating_sub(1);
                        self.update_selection.select_column(Some(column));
                    }
                    KeyCode::Char('l') | KeyCode::Right if !self.updates.is_empty() => {
                        let column = match self.update_selection.selected_column() {
                            Some(column) => (column + 1).min(4),
                            None => 0,
                        };
                        self.update_selection.select_column(Some(column));
                    }
                    KeyCode::Enter => {
                        if let Some(update) = self
                            .update_selection
                            .selected()
                            .and_then(|index| self.updates.get(index))
                        {
                            self.opened_update_id = Some(update.id.clone());
                            self.detail_scroll = 0;
                            self.show_update_table = false;
                            self.focused_pane = BrowserPane::LatestUpdate;
                        }
                    }
                    KeyCode::Char('e') => {
                        if let Some(id) = self
                            .update_selection
                            .selected()
                            .and_then(|index| self.updates.get(index))
                            .map(|update| update.id.clone())
                        {
                            self.start_update_edit(conn, text_in, &id);
                        }
                    }
                    KeyCode::Char('d') => {
                        if let Some(update) = self
                            .update_selection
                            .selected()
                            .and_then(|index| self.updates.get(index))
                        {
                            self.pending_update_delete_id = Some(update.id.clone());
                            self.err = None;
                        }
                    }
                    _ => {}
                }
            } else if self.show_help {
                match key_event.code {
                    KeyCode::Esc | KeyCode::Char('?') => self.show_help = false,
                    KeyCode::Char('q') => self.exit = true,
                    _ => {}
                }
            } else {
                match key_event.code {
                    // New Project
                    KeyCode::Char('A') => {
                        self.show_project_input = true;
                        self.err = None;
                        text_in.input_mode = InputMode::Editing;
                    }
                    // Quit
                    KeyCode::Char('q') => self.exit = true,
                    // Down on projects list
                    KeyCode::Char('j') | KeyCode::Down
                        if self.focused_pane == BrowserPane::Projects
                            && !self.projects.is_empty() =>
                    {
                        let index = match self.project_selection.selected() {
                            Some(index) => index.saturating_add(1).min(self.projects.len() - 1),
                            None => 0,
                        };

                        self.project_selection.select(Some(index));
                    }
                    // Up on projects list
                    KeyCode::Char('k') | KeyCode::Up
                        if self.focused_pane == BrowserPane::Projects
                            && !self.projects.is_empty() =>
                    {
                        let index = match self.project_selection.selected() {
                            Some(index) => index.saturating_sub(1),
                            None => 0,
                        };

                        self.project_selection.select(Some(index));
                    }
                    // Open updates table
                    KeyCode::Char('u') if !self.opened_project_id.is_none() => {
                        self.show_update_table = true;
                    }
                    // Open project
                    KeyCode::Enter if self.focused_pane == BrowserPane::Projects => {
                        if let Some(project) = self
                            .project_selection
                            .selected()
                            .and_then(|index| self.projects.get(index))
                        {
                            self.opened_project_id = Some(project.id.clone());
                            self.clear_updates();
                            if let Err(error) = self.reload_updates(conn) {
                                self.err = Some(format!(
                                    "Error retrieving updates for this project: {error}"
                                ))
                            }
                            self.focused_pane = BrowserPane::LatestUpdate;
                        }
                    }
                    // Left pane
                    KeyCode::Char('h') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.focused_pane = BrowserPane::Projects;
                    }
                    // Right pane
                    KeyCode::Char('l') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.focused_pane = BrowserPane::LatestUpdate;
                    }
                    // Delete project
                    KeyCode::Char('d')
                        if let Some(project) = self
                            .project_selection
                            .selected()
                            .and_then(|index| self.projects.get(index))
                            && self.focused_pane == BrowserPane::Projects =>
                    {
                        self.pending_project_delete_id = Some(project.id.clone());
                    }
                    KeyCode::Char('e') if self.focused_pane == BrowserPane::LatestUpdate => {
                        if let Some(id) = self.displayed_update().map(|update| update.id.clone()) {
                            self.start_update_edit(conn, text_in, &id);
                        }
                    }
                    // New update
                    KeyCode::Char('a') if !self.opened_project_id.is_none() => {
                        self.show_update_input = true;
                        self.err = None;
                        text_in.input_mode = InputMode::Editing;
                    }
                    // Help window
                    KeyCode::Char('?') => {
                        self.help_scroll = 0;
                        self.show_help = true;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}
