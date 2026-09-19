use crate::{
    types::project::ProjectStep,
    ui::input::{self, InputMode},
};

use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout, Position},
    style::{Color, Style, Stylize},
    text::{Line, Text},
    widgets::{Block, Paragraph},
};
use rusqlite::Connection;
use std::io;

#[cfg(test)]
mod tests;

pub struct App {
    pub show_project_input: bool,
    pub err: Option<String>,
    pub exit: bool,
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal, conn: &Connection) -> io::Result<()> {
        let mut text_in = input::Input::new();
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
                            text_in.project_step = ProjectStep::Confirm;
                            self.err = None;
                        }
                    }
                    KeyCode::Enter if text_in.project_step == ProjectStep::Confirm => {
                        match text_in.submit_project(conn) {
                            Ok(()) => {
                                text_in.reset_all();
                                self.show_project_input = false;
                                self.err = None;
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
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn render(&self, frame: &mut Frame, text_in: &mut input::Input) {
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
        } else {
            let layout = Layout::vertical([Constraint::Length(1)]);
            let [help_area] = frame.area().layout(&layout);

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
        }
    }
}
