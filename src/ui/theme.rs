use ratatui::style::{Color, Modifier, Style};

pub const BACKGROUND: Color = Color::Rgb(24, 29, 40);
pub const TEXT: Color = Color::Rgb(222, 228, 239);
pub const MUTED: Color = Color::Rgb(153, 166, 188);
pub const BORDER: Color = Color::Rgb(89, 104, 128);
pub const ACCENT: Color = Color::Rgb(125, 218, 202);
pub const ERROR: Color = Color::Rgb(255, 145, 155);
pub const ADDED: Color = Color::Rgb(158, 222, 140);
pub const REMOVED: Color = Color::Rgb(255, 145, 155);

pub const BASE: Style = Style::new().fg(TEXT).bg(BACKGROUND);
pub const SECONDARY: Style = Style::new().fg(MUTED);
pub const HEADING: Style = Style::new().fg(ACCENT).add_modifier(Modifier::BOLD);
pub const ROW: Style = Style::new().bg(Color::Rgb(37, 61, 73));
pub const COLUMN: Style = Style::new().bg(Color::Rgb(49, 45, 70));
pub const CELL: Style = Style::new()
    .fg(BACKGROUND)
    .bg(ACCENT)
    .add_modifier(Modifier::BOLD);

pub fn border(focused: bool) -> Style {
    Style::new().fg(if focused { ACCENT } else { BORDER })
}
