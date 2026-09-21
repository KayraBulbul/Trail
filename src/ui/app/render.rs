use super::{App, BrowserPane};
use crate::{
    types::{project::ProjectStep, update::UpdateStep},
    ui::{input, theme},
};
use chrono::Local;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Position, Rect},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{
        Block, Cell, Clear, List, ListItem, Paragraph, Row, Scrollbar, ScrollbarState, Table, Wrap,
    },
};

const INFO_TEXT: [&str; 1] =
    ["(Esc) return | (Enter) open | (j/k) row | (h/l) column | (d) delete | (e) edit | (q) quit"];
const UPDATE_ROW_HEIGHT: u16 = 4;

impl App {
    pub(super) fn render(&mut self, frame: &mut Frame, text_in: &mut input::Input) {
        frame.render_widget(Block::new().style(theme::BASE), frame.area());
        if self.show_project_input
            && (text_in.project_step == ProjectStep::Name
                || text_in.project_step == ProjectStep::Directory)
        {
            self.render_project_input(frame, text_in);
        } else if self.show_update_input
            && (text_in.update_step == UpdateStep::Title
                || text_in.update_step == UpdateStep::Body
                || text_in.update_step == UpdateStep::Next)
        {
            self.render_update_input(frame, text_in);
        } else if self.show_project_input && text_in.project_step == ProjectStep::Confirm {
            self.render_project_confirmation(frame, text_in);
        } else if self.show_update_input && text_in.update_step == UpdateStep::Confirm {
            self.render_update_confirmation(frame, text_in);
        } else if self.projects.is_empty() {
            self.render_empty_state(frame);
        } else if self.show_update_table {
            let layout = Layout::vertical([Constraint::Min(5), Constraint::Length(3)]);
            let rects = frame.area().layout_vec(&layout);

            self.render_table(frame, rects[0]);
            self.render_footer(frame, rects[1]);
        } else if self.show_help {
            self.render_help_window(frame);
        } else {
            self.render_browser(frame);
        }
        self.render_delete_confirmation(frame);
    }

    fn render_project_input(&self, frame: &mut Frame, text_in: &input::Input) {
        let (title, placeholder) = match text_in.project_step {
            ProjectStep::Name => ("Project Name", "Optional: name, defaults to directory name"),
            ProjectStep::Directory => (
                "Project Directory",
                "Required: path to your project directory",
            ),
            ProjectStep::Confirm => unreachable!("Confirm is rendered separately"),
        };
        self.render_input(frame, text_in, title, placeholder, false);
    }

    fn render_update_input(&self, frame: &mut Frame, text_in: &input::Input) {
        let (title, placeholder) = match text_in.update_step {
            UpdateStep::Title => ("Update Title", "Required: a short title for this update"),
            UpdateStep::Body => ("Update Body", "Required: what did you work on?"),
            UpdateStep::Next => ("What's Next?", "Optional: what will you work on next?"),
            UpdateStep::Confirm => unreachable!("Confirm is rendered separately"),
        };
        let multiline = matches!(text_in.update_step, UpdateStep::Body | UpdateStep::Next);
        let title = if text_in.editing_update.is_some() {
            format!("Edit: {title}")
        } else {
            title.to_string()
        };
        self.render_input(frame, text_in, &title, placeholder, multiline);
    }

    fn render_input(
        &self,
        frame: &mut Frame,
        text_in: &input::Input,
        title: &str,
        placeholder: &str,
        multiline: bool,
    ) {
        let area = frame.area();
        let width = area.width.min(72);
        if width < 3 || area.height < 5 {
            return;
        }
        let (mut lines, (cursor_x, cursor_y)) = wrapped_input(
            &text_in.input,
            text_in.character_index,
            usize::from(width - 2),
        );
        let color = if text_in.input.is_empty() {
            lines = wrapped_input(placeholder, usize::MAX, usize::from(width - 2)).0;
            theme::MUTED
        } else {
            theme::TEXT
        };
        let help = if multiline {
            "Enter: continue | Esc: cancel | Shift+Enter: new line"
        } else {
            "Enter: continue | Esc: cancel"
        };
        let help_lines = wrapped_input(help, usize::MAX, usize::from(width)).0;
        let error_height = u16::from(self.err.is_some());
        let help_height = help_lines
            .len()
            .min(usize::from(area.height - 3 - error_height)) as u16;
        let help = Paragraph::new(Text::from(
            help_lines.into_iter().map(Line::from).collect::<Vec<_>>(),
        ))
        .style(theme::SECONDARY)
        .centered();
        let height = (lines.len() + 2).min(usize::from(
            area.height.saturating_sub(help_height + error_height),
        )) as u16;
        let input_area = Rect::new(
            area.x + (area.width - width) / 2,
            area.y
                + ((area.height - height) / 2)
                    .min(area.height - height - help_height - error_height),
            width,
            height,
        );
        let scroll = (cursor_y + 1).saturating_sub(usize::from(height - 2));
        let content = Paragraph::new(Text::from(
            lines
                .into_iter()
                .skip(scroll)
                .map(Line::from)
                .collect::<Vec<_>>(),
        ))
        .fg(color);
        frame.render_widget(
            content.block(
                Block::bordered()
                    .title(title)
                    .border_style(theme::border(true)),
            ),
            input_area,
        );
        frame.render_widget(
            help,
            Rect::new(input_area.x, input_area.bottom(), width, help_height),
        );
        if let Some(error) = &self.err {
            frame.render_widget(
                Paragraph::new(error.as_str()).fg(theme::ERROR).centered(),
                Rect::new(
                    input_area.x,
                    input_area.bottom() + help_height,
                    width,
                    error_height,
                ),
            );
        }
        frame.set_cursor_position(Position::new(
            input_area.x + 1 + cursor_x as u16,
            input_area.y + 1 + (cursor_y - scroll) as u16,
        ));
    }

    fn render_project_confirmation(&self, frame: &mut Frame, text_in: &input::Input) {
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
                Paragraph::new(error.to_string()).style(Style::default().fg(theme::ERROR));
            frame.render_widget(message, error_area);
        }

        let directory = text_in.project.directory.as_deref().unwrap_or("(missing)");
        let name = text_in.project.name.as_deref().unwrap_or("(missing)");

        let name_msg = vec!["Name: ".into(), name.bold()];
        let directory_msg = vec!["Directory: ".into(), directory.bold()];

        let help_text = Text::from(Line::from(help_msg)).patch_style(Style::default());
        let help_message = Paragraph::new(help_text).style(theme::SECONDARY);
        let name_text = Text::from(Line::from(name_msg)).patch_style(Style::default());
        let name_message = Paragraph::new(name_text);
        let directory_text = Text::from(Line::from(directory_msg)).patch_style(Style::default());
        let directory_message = Paragraph::new(directory_text);

        frame.render_widget(help_message, help_area);
        frame.render_widget(name_message, name_area);
        frame.render_widget(directory_message, directory_area);
    }

    fn render_update_confirmation(&self, frame: &mut Frame, text_in: &input::Input) {
        let layout = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ]);
        let [help_area, details_area, error_area] = frame.area().layout(&layout);

        let help_msg = vec![
            "Press ".into(),
            "Esc".bold(),
            " to abort, ".into(),
            "Enter".bold(),
            if text_in.editing_update.is_some() {
                " to save changes."
            } else {
                " to confirm update."
            }
            .into(),
        ];

        if let Some(error) = &self.err {
            let message =
                Paragraph::new(error.to_string()).style(Style::default().fg(theme::ERROR));
            frame.render_widget(message, error_area);
        }

        let title = text_in.update.title.as_deref().unwrap_or("(missing)");
        let body = text_in.update.body.as_deref().unwrap_or("(missing)");
        let next = text_in.update.next.as_deref().unwrap_or("(missing)");

        let help_text = Text::from(Line::from(help_msg)).patch_style(Style::default());
        let help_message = Paragraph::new(help_text).style(theme::SECONDARY);
        let details = Paragraph::new(format!("Title: {title}\nBody: {body}\nNext: {next}"))
            .wrap(Wrap { trim: false });

        frame.render_widget(help_message, help_area);
        frame.render_widget(details, details_area);
    }

    fn render_empty_state(&self, frame: &mut Frame) {
        let error_height = if self.err.is_some() { 1 } else { 0 };

        let layout = Layout::vertical([Constraint::Length(error_height), Constraint::Length(3)]);
        let [error_area, help_area] = frame.area().layout(&layout);

        if let Some(error) = &self.err {
            let message = Paragraph::new(error.as_str()).style(Style::default().fg(theme::ERROR));

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
        let help_message = Paragraph::new(text).block(
            Block::bordered()
                .title("Get Started!")
                .border_style(theme::border(true)),
        );
        frame.render_widget(help_message, help_area);
    }

    fn render_browser(&mut self, frame: &mut Frame) {
        let error_height = if self.err.is_some() { 1 } else { 0 };

        let [content_area, error_area, help_area] = frame.area().layout(&Layout::vertical([
            Constraint::Min(0),
            Constraint::Length(error_height),
            Constraint::Length(1),
        ]));

        if let Some(error) = &self.err {
            let message = Paragraph::new(error.as_str()).style(Style::default().fg(theme::ERROR));

            frame.render_widget(message, error_area);
        }

        let [project_list_area, latest_update_area] = content_area.layout(&Layout::horizontal([
            Constraint::Percentage(20),
            Constraint::Percentage(80),
        ]));

        let projects: Vec<ListItem<'_>> = self
            .projects
            .iter()
            .map(|project| {
                let item = ListItem::new(project.name.as_str());
                if self.opened_project_id.as_deref() == Some(project.id.as_str()) {
                    item.style(theme::ROW.fg(theme::ACCENT))
                } else {
                    item
                }
            })
            .collect();

        let projects_focused = self.focused_pane == BrowserPane::Projects;
        let list = List::new(projects).highlight_symbol("> ").block(
            Block::bordered()
                .title("Projects")
                .border_style(theme::border(projects_focused)),
        );
        frame.render_stateful_widget(list, project_list_area, &mut self.project_selection);

        if let Some(update) = self.displayed_update() {
            let created_at_local_time = update.created_at.with_timezone(&Local);
            let updated_at_local_time = update.updated_at.with_timezone(&Local);

            let created_at_display = created_at_local_time.format("%d %b %Y, %H:%M").to_string();
            let updated_at_display = updated_at_local_time.format("%d %b %Y, %H:%M").to_string();

            let created_at_msg = vec!["Created at: ".into(), created_at_display.bold()];
            let updated_at_msg = vec!["Updated at: ".into(), updated_at_display.bold()];

            let mut text = Text::from(format!(
                "Title: {}\nBody: {}\nWhat to do next: {}",
                update.title, update.body, update.next,
            ));
            text.lines.extend([
                Line::from(created_at_msg).style(theme::SECONDARY),
                Line::from(updated_at_msg).style(theme::SECONDARY),
            ]);

            let details = Paragraph::new(text).wrap(Wrap { trim: false }).block(
                Block::bordered()
                    .title(update.title.as_str())
                    .border_style(theme::border(
                        self.focused_pane == BrowserPane::LatestUpdate,
                    )),
            );

            frame.render_widget(details, latest_update_area);
        } else {
            if self.opened_project_id.is_none() {
                let (msg, style) = (
                    vec!["Select a project and press ".into(), "Enter".bold()],
                    Style::default(),
                );
                let text = Text::from(Line::from(msg)).patch_style(style);
                let project_message = Paragraph::new(text).style(theme::SECONDARY).block(
                    Block::bordered().border_style(theme::border(
                        self.focused_pane == BrowserPane::LatestUpdate,
                    )),
                );
                frame.render_widget(project_message, latest_update_area);
            } else if self.updates.is_empty() {
                let (msg, style) = (
                    vec![
                        "There are currently no updates for this project, Press ".into(),
                        "a".bold(),
                        " to create one!".into(),
                    ],
                    Style::default(),
                );

                let text = Text::from(Line::from(msg)).patch_style(style);
                let project_message =
                    Paragraph::new(text)
                        .wrap(Wrap { trim: false })
                        .style(theme::SECONDARY)
                        .block(Block::bordered().title("Latest Update").border_style(
                            theme::border(self.focused_pane == BrowserPane::LatestUpdate),
                        ));
                frame.render_widget(project_message, latest_update_area);
            }
        }

        let help = match self.focused_pane {
            BrowserPane::Projects => {
                "(A) new | (Enter) open | (j/k) select | (d) delete | (Ctrl+l) updates | (?) help | (q) quit"
            }
            BrowserPane::LatestUpdate => {
                "(a) new update | (e) edit | (u) table | (Ctrl+h) projects | (?) help | (q) quit"
            }
        };
        let help_message = Paragraph::new(help).style(theme::SECONDARY);
        frame.render_widget(help_message, help_area);
    }

    fn render_delete_confirmation(&self, frame: &mut Frame) {
        let (title, name, description, note) =
            if let Some(project) = self.projects.iter().find(|project| {
                self.pending_project_delete_id.as_deref() == Some(project.id.as_str())
            }) {
                (
                    "Delete project?",
                    project.name.as_str(),
                    "This removes the project from Trail.",
                    "Files on disk will remain.",
                )
            } else if let Some(update) = self
                .updates
                .iter()
                .find(|update| self.pending_update_delete_id.as_deref() == Some(update.id.as_str()))
            {
                (
                    "Delete update?",
                    update.title.as_str(),
                    "This removes the update from Trail.",
                    "This cannot be undone.",
                )
            } else {
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
            Line::from(name.bold()),
            Line::from(""),
            Line::from(description),
            Line::from(note),
            Line::from(""),
            Line::from(vec![
                "Enter".bold(),
                ": delete    ".into(),
                "Esc".bold(),
                ": cancel".into(),
            ]),
        ])
        .style(theme::BASE)
        .block(
            Block::bordered()
                .title(title)
                .border_style(Style::new().fg(theme::ERROR)),
        );

        frame.render_widget(Clear, popup);
        frame.render_widget(message, popup);
    }

    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        if self.updates.is_empty() {
            frame.render_widget(
                Paragraph::new("No updates are available for this project.")
                    .style(theme::SECONDARY),
                area,
            );
            return;
        }

        let [table_area, scrollbar_area] = area.layout(&Layout::horizontal([
            Constraint::Min(0),
            Constraint::Length(1),
        ]));
        let visible_rows = usize::from(table_area.height.saturating_sub(1) / UPDATE_ROW_HEIGHT);
        let max_offset = self.updates.len().saturating_sub(visible_rows);
        *self.update_selection.offset_mut() = self.update_selection.offset().min(max_offset);

        let header = ["Title", "Body", "Next", "Created At", "Updated At"]
            .into_iter()
            .enumerate()
            .map(|(column, title)| {
                Cell::from(title).style(
                    if self.update_selection.selected_column() == Some(column) {
                        theme::CELL
                    } else {
                        theme::HEADING
                    },
                )
            })
            .collect::<Row>()
            .height(1);
        let rows = self.updates.iter().map(|data| {
            [
                Cell::from(format!("\n{}\n", data.title)),
                Cell::from(format!("\n{}\n", data.body)),
                Cell::from(format!("\n{}\n", data.next)),
                Cell::from(format!("\n{}\n", data.created_at)),
                Cell::from(format!("\n{}\n", data.updated_at)),
            ]
            .into_iter()
            .collect::<Row>()
            .style(Style::new())
            .height(UPDATE_ROW_HEIGHT)
        });
        let bar = " █ ";
        let t = Table::new(
            rows,
            [
                Constraint::Percentage(20),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(15),
                Constraint::Percentage(15),
            ],
        )
        .header(header)
        .row_highlight_style(theme::ROW)
        .column_highlight_style(theme::COLUMN)
        .cell_highlight_style(theme::CELL)
        .highlight_symbol(Text::from(vec![
            "".into(),
            bar.into(),
            bar.into(),
            "".into(),
        ]))
        .highlight_spacing(ratatui::widgets::HighlightSpacing::Always);
        frame.render_stateful_widget(t, table_area, &mut self.update_selection);

        let [_, scrollbar_area] = scrollbar_area.layout(&Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
        ]));
        if visible_rows == 0 || self.updates.len() <= visible_rows {
            return;
        }
        // Ratatui counts scroll positions, including the final viewport, in content_length.
        let mut scroll_state = ScrollbarState::new(max_offset + 1)
            .position(self.update_selection.offset())
            .viewport_content_length(visible_rows);
        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ratatui::widgets::ScrollbarOrientation::VerticalRight)
                .thumb_style(Style::new().fg(theme::ACCENT))
                .track_style(Style::new().fg(theme::BORDER))
                .begin_symbol(None)
                .end_symbol(None),
            scrollbar_area,
            &mut scroll_state,
        );
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let info_footer = Paragraph::new(self.err.as_deref().unwrap_or(INFO_TEXT[0]))
            .style(if self.err.is_some() {
                Style::new().fg(theme::ERROR)
            } else {
                theme::SECONDARY
            })
            .centered()
            .block(Block::bordered().border_style(theme::border(false)));

        frame.render_widget(info_footer, area);
    }

    fn render_help_window(&self, frame: &mut Frame) {
        let help = Paragraph::new(vec![
            Line::from("Browser").style(theme::HEADING),
            Line::from("A: new project"),
            Line::from("?: help"),
            Line::from("q: quit"),
            Line::from("j / Down, k / Up: select a project (Projects focused)"),
            Line::from("Enter: open selected project (Projects focused)"),
            Line::from("Ctrl+h: focus Projects"),
            Line::from("Ctrl+l: focus Latest Update"),
            Line::from("d: delete selected project (Projects focused)"),
            Line::from("a: new update"),
            Line::from("e: edit displayed update (Latest Update focused)"),
            Line::from("u: update table (requires an open project)"),
            Line::from(""),
            Line::from("Update table").style(theme::HEADING),
            Line::from("j / Down, k / Up: select row"),
            Line::from("h / Left, l / Right: select column"),
            Line::from("Enter: open selected update"),
            Line::from("d: delete selected update"),
            Line::from("e: edit selected update"),
            Line::from("Esc: return"),
            Line::from("q: quit"),
            Line::from(""),
            Line::from("Project and update forms").style(theme::HEADING),
            Line::from("Type: insert text"),
            Line::from("Left / Right: move cursor"),
            Line::from("Backspace: delete previous character"),
            Line::from("Enter: submit field"),
            Line::from("Shift+Enter: new line in update body / next"),
            Line::from("Esc: cancel"),
            Line::from(""),
            Line::from("Delete confirmation").style(theme::HEADING),
            Line::from("Enter: confirm deletion"),
            Line::from("Esc: cancel"),
            Line::from(""),
            Line::from("Help").style(theme::HEADING),
            Line::from("Esc / ?: return"),
            Line::from("q: quit"),
        ])
        .block(
            Block::bordered()
                .title("Trail keybindings")
                .border_style(theme::border(true)),
        )
        .wrap(Wrap { trim: false });

        frame.render_widget(help, frame.area());
    }
}

// Wrap text and locate the cursor together so both use the same terminal cell widths.
fn wrapped_input(
    text: &str,
    character_index: usize,
    width: usize,
) -> (Vec<String>, (usize, usize)) {
    let mut lines = Vec::new();
    let mut cursor = (0, 0);
    let mut index = 0;
    for line in text.split('\n') {
        lines.push(String::new());
        let mut x = 0;
        let span = Span::raw(line);
        for grapheme in span.styled_graphemes(Style::default()) {
            let cell_width = Span::raw(grapheme.symbol).width();
            if x + cell_width > width {
                lines.push(String::new());
                x = 0;
            }
            let next_index = index + grapheme.symbol.chars().count();
            if (index..next_index).contains(&character_index) {
                cursor = (x, lines.len() - 1);
            }
            lines.last_mut().unwrap().push_str(grapheme.symbol);
            x += cell_width;
            index = next_index;
        }
        if index == character_index {
            if x >= width {
                lines.push(String::new());
                x = 0;
            }
            cursor = (x, lines.len() - 1);
        }
        index += 1;
    }
    (lines, cursor)
}
