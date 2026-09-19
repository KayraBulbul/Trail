use crate::{
    database::sqlite,
    types::project::{Project, ProjectStep},
    ui::input::{self, InputMode},
};

use chrono::Local;
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout, Position, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Text},
    widgets::{Block, Clear, List, ListItem, ListState, Paragraph},
};
use rusqlite::{Connection, Result};
use std::io;

#[cfg(test)]
mod tests;

#[derive(PartialEq)]
pub enum BrowserPane {
    Projects,
    Project,
}

pub struct App {
    pub show_project_input: bool,
    pub projects: Vec<Project>,
    pub project_selection: ListState,
    pub opened_project_id: Option<String>,
    pub pending_delete_id: Option<String>,
    pub focused_pane: BrowserPane,
    pub err: Option<String>,
    pub exit: bool,
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal, conn: &Connection) -> io::Result<()> {
        let mut text_in = input::Input::new();

        if let Err(err) = self.reload_projects(conn) {
            self.err = Some(format!("Couldn't load projects: {err}"));
        }

        while !self.exit {
            terminal.draw(|frame| self.render(frame, &mut text_in))?;
            match crossterm::event::read()? {
                Event::Key(key_event) => self.handle_key_event(key_event, &mut text_in, conn)?,
                _ => {}
            }
        }

        Ok(())
    }

    fn handle_key_event(
        &mut self,
        key_event: crossterm::event::KeyEvent,
        text_in: &mut input::Input,
        conn: &Connection,
    ) -> io::Result<()> {
        if key_event.kind == KeyEventKind::Press {
            if self.pending_delete_id.is_some() {
                match key_event.code {
                    KeyCode::Enter => {
                        if let Some(id) = self.pending_delete_id.take() {
                            self.delete_project(conn, &id);
                        }
                    }
                    KeyCode::Esc => self.pending_delete_id = None,
                    _ => {}
                }
                return Ok(());
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
                            text_in.project_step = ProjectStep::Confirm;
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
                            Ok(()) => {
                                self.err = None;
                                if let Err(err) = self.reload_projects(conn) {
                                    self.err = Some(format!("Couldn't load projects: {err}"));
                                }
                                text_in.reset_all();
                                self.show_project_input = false;
                                self.project_selection.select(Some(0));
                                if let Some(project) = self
                                    .project_selection
                                    .selected()
                                    .and_then(|index| self.projects.get(index))
                                {
                                    self.opened_project_id = Some(project.id.clone());
                                }
                                self.focused_pane = BrowserPane::Project;
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
            } else {
                match key_event.code {
                    KeyCode::Char('A') => {
                        self.show_project_input = true;
                        text_in.input_mode = InputMode::Editing;
                    }
                    KeyCode::Char('q') => self.exit = true,
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
                    KeyCode::Enter => {
                        if let Some(project) = self
                            .project_selection
                            .selected()
                            .and_then(|index| self.projects.get(index))
                        {
                            self.opened_project_id = Some(project.id.clone());
                        }
                    }
                    KeyCode::Char('h') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.focused_pane = BrowserPane::Projects;
                    }
                    KeyCode::Char('l') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                        self.focused_pane = BrowserPane::Project;
                    }
                    KeyCode::Char('D')
                        if let Some(project) = self
                            .project_selection
                            .selected()
                            .and_then(|index| self.projects.get(index))
                            && self.focused_pane == BrowserPane::Projects =>
                    {
                        self.pending_delete_id = Some(project.id.clone());
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn render(&mut self, frame: &mut Frame, text_in: &mut input::Input) {
        if self.show_project_input
            && (text_in.project_step == ProjectStep::Name
                || text_in.project_step == ProjectStep::Directory)
        {
            let layout = Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Length(1),
            ]);
            let [help_area, input_area, error_area] = frame.area().layout(&layout);

            let (msg, style) = match text_in.project_step {
                ProjectStep::Name => (
                    vec![
                        "Press ".into(),
                        "Esc".bold(),
                        " to stop editing, ".into(),
                        "Enter".bold(),
                        " your project name:".into(),
                    ],
                    Style::default(),
                ),
                ProjectStep::Directory => (
                    vec![
                        "Press ".into(),
                        "Esc".bold(),
                        " to stop editing, ".into(),
                        "Enter".bold(),
                        " your project directory:".into(),
                    ],
                    Style::default(),
                ),
                ProjectStep::Confirm => unreachable!("Confirm is rendered separately"),
            };
            let text = Text::from(Line::from(msg)).patch_style(style);
            let help_message = Paragraph::new(text);
            frame.render_widget(help_message, help_area);

            let input = Paragraph::new(text_in.input.as_str())
                .style(Style::default().fg(Color::Yellow))
                .block(Block::bordered().title(match text_in.project_step {
                    ProjectStep::Name => "Project Name",
                    ProjectStep::Directory => "Project Directory",
                    ProjectStep::Confirm => unreachable!("Confirm is rendered separately"),
                }));

            if let Some(error) = &self.err {
                let message =
                    Paragraph::new(error.to_string()).style(Style::default().fg(Color::Red));
                frame.render_widget(message, error_area);
            }

            frame.render_widget(input, input_area);
            #[expect(clippy::cast_possible_truncation)]
            frame.set_cursor_position(Position::new(
                input_area.x + text_in.character_index as u16 + 1,
                input_area.y + 1,
            ));
        } else if self.show_project_input && text_in.project_step == ProjectStep::Confirm {
            let layout = Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
            ]);
            let [help_area, name_area, directory_area, error_area] = frame.area().layout(&layout);

            let help_msg = vec![
                "Press ".into(),
                "Esc".bold(),
                " to abort, ".into(),
                "Enter".bold(),
                " to confirm project.".into(),
            ];

            if let Some(error) = &self.err {
                let message =
                    Paragraph::new(error.to_string()).style(Style::default().fg(Color::Red));
                frame.render_widget(message, error_area);
            }

            let directory = text_in.project.directory.as_deref().unwrap_or("(missing)");
            let name = text_in.project.name.as_deref().unwrap_or("(missing)");

            let name_msg = vec!["Name: ".into(), name.bold()];
            let directory_msg = vec!["Directory: ".into(), directory.bold()];

            let help_text = Text::from(Line::from(help_msg)).patch_style(Style::default());
            let help_message = Paragraph::new(help_text);
            let name_text = Text::from(Line::from(name_msg)).patch_style(Style::default());
            let name_message = Paragraph::new(name_text);
            let directory_text =
                Text::from(Line::from(directory_msg)).patch_style(Style::default());
            let directory_message = Paragraph::new(directory_text);

            frame.render_widget(help_message, help_area);
            frame.render_widget(name_message, name_area);
            frame.render_widget(directory_message, directory_area);
        } else if self.projects.is_empty() {
            let error_height = if self.err.is_some() { 1 } else { 0 };

            let layout =
                Layout::vertical([Constraint::Length(error_height), Constraint::Length(1)]);
            let [error_area, help_area] = frame.area().layout(&layout);

            if let Some(error) = &self.err {
                let message = Paragraph::new(error.as_str()).style(Style::default().fg(Color::Red));

                frame.render_widget(message, error_area);
            }

            let (msg, style) = (
                vec![
                    "Press ".into(),
                    "A".bold(),
                    " to create a new project, ".into(),
                    "q".bold(),
                    " to quit.".into(),
                ],
                Style::default(),
            );
            let text = Text::from(Line::from(msg)).patch_style(style);
            let help_message = Paragraph::new(text);
            frame.render_widget(help_message, help_area);
        } else {
            let error_height = if self.err.is_some() { 1 } else { 0 };

            let [content_area, error_area, help_area] = frame.area().layout(&Layout::vertical([
                Constraint::Min(0),
                Constraint::Length(error_height),
                Constraint::Length(1),
            ]));

            if let Some(error) = &self.err {
                let message = Paragraph::new(error.as_str()).style(Style::default().fg(Color::Red));

                frame.render_widget(message, error_area);
            }

            let [project_list_area, project_area] = content_area.layout(&Layout::horizontal([
                Constraint::Percentage(30),
                Constraint::Percentage(70),
            ]));

            let items: Vec<ListItem<'_>> = self
                .projects
                .iter()
                .map(|project| ListItem::new(project.name.as_str()))
                .collect();

            let projects_border_colour = if self.focused_pane == BrowserPane::Projects {
                Color::Yellow
            } else {
                Color::Reset
            };

            let list = List::new(items).highlight_symbol("> ").block(
                Block::bordered()
                    .title("Projects")
                    .border_style(Style::default().fg(projects_border_colour)),
            );
            frame.render_stateful_widget(list, project_list_area, &mut self.project_selection);

            let opened_project = self
                .projects
                .iter()
                .find(|project| self.opened_project_id.as_deref() == Some(project.id.as_str()));

            if let Some(project) = opened_project {
                let id = vec!["ID: ".into(), project.id.as_str().bold()];
                let name_msg = vec!["Name: ".into(), project.name.as_str().bold()];
                let directory_msg = vec!["Directory: ".into(), project.directory.as_str().bold()];

                let created_at_local_time = project.created_at.with_timezone(&Local);
                let updated_at_local_time = project.updated_at.with_timezone(&Local);

                let created_at_display =
                    created_at_local_time.format("%d %b %Y, %H:%M").to_string();
                let updated_at_display =
                    updated_at_local_time.format("%d %b %Y, %H:%M").to_string();

                let created_at_msg = vec!["Created at: ".into(), created_at_display.bold()];
                let updated_at_msg = vec!["Updated at: ".into(), updated_at_display.bold()];

                let text = Text::from(vec![
                    Line::from(id),
                    Line::from(name_msg),
                    Line::from(directory_msg),
                    Line::from(created_at_msg),
                    Line::from(updated_at_msg),
                ]);

                let project_border_colour = if self.focused_pane == BrowserPane::Project {
                    Color::Yellow
                } else {
                    Color::Reset
                };

                let details = Paragraph::new(text).block(
                    Block::bordered()
                        .title(project.name.as_str())
                        .border_style(Style::default().fg(project_border_colour)),
                );

                frame.render_widget(details, project_area);
            } else {
                let (msg, style) = (
                    vec!["Select a project and press ".into(), "Enter".bold()],
                    Style::default(),
                );
                let text = Text::from(Line::from(msg)).patch_style(style);
                let project_message = Paragraph::new(text);
                frame.render_widget(project_message, project_area);
            }

            let (msg, style) = (
                vec![
                    "Press ".into(),
                    "A".bold(),
                    " to create a new project, ".into(),
                    "j/k".bold(),
                    " for down/up, ".into(),
                    "Enter".bold(),
                    " to open selected project, ".into(),
                    "CTRL + h/l".bold(),
                    " to switch panes, ".into(),
                    "D".bold(),
                    " to delete project, ".into(),
                    "q".bold(),
                    " to quit.".into(),
                ],
                Style::default(),
            );
            let text = Text::from(Line::from(msg)).patch_style(style);
            let help_message = Paragraph::new(text);
            frame.render_widget(help_message, help_area);
        }
        self.render_delete_confirmation(frame);
    }

    fn render_delete_confirmation(&self, frame: &mut Frame) {
        let Some(project) = self
            .projects
            .iter()
            .find(|project| self.pending_delete_id.as_deref() == Some(project.id.as_str()))
        else {
            return;
        };

        let area = frame.area();
        let width = area.width.min(60);
        let height = area.height.min(8);
        let popup = Rect::new(
            area.x + (area.width - width) / 2,
            area.y + (area.height - height) / 2,
            width,
            height,
        );
        let message = Paragraph::new(vec![
            Line::from(project.name.as_str().bold()),
            Line::from(""),
            Line::from("This removes the project from Trail."),
            Line::from("Files on disk will remain."),
            Line::from(""),
            Line::from(vec![
                "Enter".bold(),
                ": delete   ".into(),
                "Esc".bold(),
                ": return".into(),
            ]),
        ])
        .block(Block::bordered().title("Delete project?"));

        frame.render_widget(Clear, popup);
        frame.render_widget(message, popup);
    }

    fn delete_project(&mut self, conn: &Connection, id: &str) {
        if let Err(error) = sqlite::delete_project(conn, id) {
            self.err = Some(format!("Unable to delete project: {error}"));
            return;
        }

        self.err = None;
        if self.opened_project_id.as_deref() == Some(id) {
            self.opened_project_id = None;
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

    fn reload_projects(&mut self, conn: &Connection) -> Result<()> {
        let projects = sqlite::get_projects(conn)?;

        let selected = if projects.is_empty() { None } else { Some(0) };

        self.projects = projects;
        self.project_selection.select(selected);

        Ok(())
    }
}
