mod input;

use std::io;

use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout, Position},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Text},
    widgets::{Block, Paragraph},
};

use crate::input::InputMode;

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();

    let mut app = App {
        show_project_input: false,
        exit: false,
    };

    let app_result = app.run(&mut terminal);

    ratatui::restore();
    app_result
}

pub struct App {
    show_project_input: bool,
    exit: bool,
}

impl App {
    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let mut text_in = input::Input::new();
        while !self.exit {
            if self.show_project_input {
                self.new_project(terminal, &mut text_in)?;
            }
            match crossterm::event::read()? {
                Event::Key(key_event) => self.handle_key_event(key_event, &mut text_in)?,
                _ => {}
            }
        }

        Ok(())
    }

    fn handle_key_event(
        &mut self,
        key_event: crossterm::event::KeyEvent,
        text_in: &mut input::Input,
    ) -> io::Result<()> {
        if key_event.kind == KeyEventKind::Press {
            if self.show_project_input && text_in.input_mode == InputMode::Editing {
                match key_event.code {
                    KeyCode::Enter => text_in.submit_project(),
                    KeyCode::Char(to_insert) => text_in.enter_char(to_insert),
                    KeyCode::Backspace => text_in.delete_char(),
                    KeyCode::Left => text_in.move_cursor_left(),
                    KeyCode::Right => text_in.move_cursor_right(),
                    KeyCode::Esc => text_in.input_mode = InputMode::Normal,
                    _ => {}
                }
            } else {
                match key_event.code {
                    KeyCode::Char('a') => {
                        self.show_project_input = true;
                    }
                    KeyCode::Char('e') => text_in.input_mode = InputMode::Editing,
                    KeyCode::Char('q') => self.exit = true,
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn new_project(
        &self,
        terminal: &mut DefaultTerminal,
        text_in: &mut input::Input,
    ) -> io::Result<()> {
        terminal.draw(|frame| self.render(frame, text_in))?;

        Ok(())
    }

    fn render(&self, frame: &mut Frame, text_in: &mut input::Input) {
        let layout = Layout::vertical([Constraint::Length(1), Constraint::Length(3)]);
        let [help_area, input_area] = frame.area().layout(&layout);

        let (msg, style) = match text_in.input_mode {
            InputMode::Normal => (
                vec![
                    "Press ".into(),
                    "q".bold(),
                    " to exit, ".into(),
                    "e".bold(),
                    " to start editing.".bold(),
                ],
                Style::default().add_modifier(Modifier::RAPID_BLINK),
            ),
            InputMode::Editing => (
                vec![
                    "Press ".into(),
                    "Esc".bold(),
                    " to stop editing, ".into(),
                    "Enter".bold(),
                    " to record the message".into(),
                ],
                Style::default(),
            ),
        };

        let text = Text::from(Line::from(msg)).patch_style(style);
        let help_message = Paragraph::new(text);
        frame.render_widget(help_message, help_area);

        let input = Paragraph::new(text_in.input.as_str())
            .style(match text_in.input_mode {
                InputMode::Normal => Style::default(),
                InputMode::Editing => Style::default().fg(Color::Yellow),
            })
            .block(Block::bordered().title("Project Name"));
        frame.render_widget(input, input_area);
        match text_in.input_mode {
            InputMode::Normal => {}
            #[expect(clippy::cast_possible_truncation)]
            InputMode::Editing => frame.set_cursor_position(Position::new(
                input_area.x + text_in.character_index as u16 + 1,
                input_area.y + 1,
            )),
        }
    }
}
