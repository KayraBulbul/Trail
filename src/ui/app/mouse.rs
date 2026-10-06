use super::{App, BrowserPane, GitPane};
use crate::ui::input;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    layout::{Margin, Position, Rect},
    style::Modifier,
    text::{Line, Span},
};
use rusqlite::Connection;
use std::io;

/// What a click on part of the screen does. `render` records these each frame.
#[derive(Clone, Copy)]
pub enum Click {
    /// A key hint, or a key word in a prompt: clicking presses the key.
    Key(KeyEvent),
    /// A pane: clicking or scrolling over it first presses the key that focuses it.
    /// Screens with panes only scroll the pane under the cursor.
    Focus(KeyEvent),
    Project(usize),
    Update(usize),
    GitItem(usize),
}

impl App {
    pub(super) fn handle_mouse_event(
        &mut self,
        mouse: MouseEvent,
        text_in: &mut input::Input,
        conn: &Connection,
    ) -> io::Result<()> {
        let position = Position::new(mouse.column, mouse.row);
        let mut hits = self
            .clicks
            .iter()
            .rev()
            .filter(|(area, _)| area.contains(position))
            .map(|&(_, click)| click);

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => match hits.next() {
                Some(Click::Key(key) | Click::Focus(key)) => {
                    self.handle_key_event(key, text_in, conn)?;
                }
                Some(Click::Project(index)) => {
                    self.focused_pane = BrowserPane::Projects;
                    self.project_selection.select(Some(index));
                    self.press(KeyCode::Enter, text_in, conn)?;
                }
                Some(Click::Update(index)) => {
                    if self.update_click == Some(index) {
                        self.press(KeyCode::Enter, text_in, conn)?;
                    } else {
                        self.update_selection.select(Some(index));
                        self.update_click = Some(index);
                    }
                }
                Some(Click::GitItem(index)) => {
                    self.git.focused_pane = GitPane::List;
                    self.git_list_selection().0.select(Some(index));
                    self.press(KeyCode::Enter, text_in, conn)?;
                }
                None => {}
            },
            MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
                let focus = hits.find_map(|click| match click {
                    Click::Focus(key) => Some(key),
                    _ => None,
                });
                let has_panes = self
                    .clicks
                    .iter()
                    .any(|(_, click)| matches!(click, Click::Focus(_)));
                match focus {
                    Some(key) => self.handle_key_event(key, text_in, conn)?,
                    // On a screen with panes, scroll only the one under the cursor.
                    None if has_panes => return Ok(()),
                    None => {}
                }
                let code = if mouse.kind == MouseEventKind::ScrollDown {
                    KeyCode::Down
                } else {
                    KeyCode::Up
                };
                self.press(code, text_in, conn)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn press(
        &mut self,
        code: KeyCode,
        text_in: &mut input::Input,
        conn: &Connection,
    ) -> io::Result<()> {
        self.handle_key_event(KeyEvent::new(code, KeyModifiers::NONE), text_in, conn)
    }

    /// Makes each hint in a help row clickable. `text` is drawn from the left
    /// edge of `area`'s row. Hints are separated by ` | ` and look like
    /// `(Enter) open`; hints for several keys (`j/k`) aren't clickable.
    pub(super) fn record_hints(&mut self, text: &str, area: Rect) {
        let mut x = area.x;
        for (i, hint) in text.split(" | ").enumerate() {
            if i > 0 {
                x += 3;
            }
            let hint_width = Span::raw(hint).width() as u16;
            let label = hint
                .strip_prefix('(')
                .and_then(|rest| rest.split(')').next());
            if let Some(key) = label.and_then(parse_key) {
                self.add_click(Rect::new(x, area.y, hint_width, 1), area, Click::Key(key));
            }
            x += hint_width;
        }
    }

    /// Makes the bold key words in a prompt line (`Press A to add one`) clickable.
    /// `line` is drawn from the left edge of `area`'s row.
    pub(super) fn record_bold_keys(&mut self, line: &Line, area: Rect) {
        let mut x = area.x;
        for span in &line.spans {
            let width = span.width() as u16;
            if span.style.add_modifier.contains(Modifier::BOLD)
                && let Some(key) = parse_key(&span.content)
            {
                self.add_click(Rect::new(x, area.y, width, 1), area, Click::Key(key));
            }
            x += width;
        }
    }

    /// Makes a popup's hint line clickable, and only it: the popup covers
    /// everything else. `keys` is the sixth line inside the popup's border.
    pub(super) fn record_popup_keys(&mut self, keys: &str, popup: Rect) {
        self.clicks.clear();
        let inner = popup.inner(Margin::new(1, 1));
        if inner.height > 5 {
            self.record_hints(keys, Rect::new(inner.x, inner.y + 5, inner.width, 1));
        }
    }

    /// Records a click target for each visible row of a bordered list drawn in `area`.
    pub(super) fn record_list_rows(
        &mut self,
        area: Rect,
        offset: usize,
        len: usize,
        click: fn(usize) -> Click,
    ) {
        let inner = area.inner(Margin::new(1, 1));
        for row in 0..inner.height {
            let index = offset + usize::from(row);
            if index >= len {
                break;
            }
            self.add_click(
                Rect::new(inner.x, inner.y + row, inner.width, 1),
                inner,
                click(index),
            );
        }
    }

    /// Records a click target, cut down to the part inside `bounds`.
    pub(super) fn add_click(&mut self, area: Rect, bounds: Rect, click: Click) {
        let area = area.intersection(bounds);
        if !area.is_empty() {
            self.clicks.push((area, click));
        }
    }
}

pub(super) fn ctrl(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
}

/// The key a hint label names (`Enter`, `Ctrl+l`, `A`). Labels for several keys
/// (`j/k`) name none.
fn parse_key(label: &str) -> Option<KeyEvent> {
    let (modifiers, key) = if let Some(key) = label.strip_prefix("Ctrl+") {
        (KeyModifiers::CONTROL, key)
    } else if let Some(key) = label.strip_prefix("Shift+") {
        (KeyModifiers::SHIFT, key)
    } else {
        (KeyModifiers::NONE, label)
    };
    let code = match key {
        "Enter" => KeyCode::Enter,
        "Esc" => KeyCode::Esc,
        "Backspace" => KeyCode::Backspace,
        "Tab" => KeyCode::Tab,
        "Left" => KeyCode::Left,
        "Right" => KeyCode::Right,
        _ => {
            let mut chars = key.chars();
            let ch = chars.next()?;
            if chars.next().is_some() {
                return None;
            }
            KeyCode::Char(ch)
        }
    };
    Some(KeyEvent::new(code, modifiers))
}
