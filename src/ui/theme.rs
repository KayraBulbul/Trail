//! Colours from the Omarchy theme "Vigil".

use ratatui::style::{Color, Modifier, Style};

pub const BACKGROUND: Color = Color::Rgb(0x19, 0x19, 0x1D);
pub const TEXT: Color = Color::Rgb(0xC5, 0xBD, 0xB6);
pub const BRIGHT: Color = Color::Rgb(0xDD, 0xD3, 0xC9);
pub const MUTED: Color = Color::Rgb(0x89, 0x81, 0x81);
pub const BORDER: Color = Color::Rgb(0x5A, 0x55, 0x5E);
pub const ACCENT: Color = Color::Rgb(0xC6, 0x5D, 0x2E);
pub const ERROR: Color = Color::Rgb(0xCA, 0x8D, 0x8D);
pub const ADDED: Color = Color::Rgb(0xA3, 0xAD, 0x9D);
pub const REMOVED: Color = Color::Rgb(0xCA, 0x8D, 0x8D);

pub const BASE: Style = Style::new().fg(TEXT).bg(BACKGROUND);
pub const SECONDARY: Style = Style::new().fg(MUTED);
pub const HEADING: Style = Style::new().fg(ACCENT).add_modifier(Modifier::BOLD);
pub const ROW: Style = Style::new().bg(Color::Rgb(0x48, 0x34, 0x2F));
pub const CELL: Style = Style::new()
    .fg(BACKGROUND)
    .bg(ACCENT)
    .add_modifier(Modifier::BOLD);

pub fn border(focused: bool) -> Style {
    Style::new().fg(if focused { ACCENT } else { BORDER })
}
