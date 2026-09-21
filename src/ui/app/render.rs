use super::{App, BrowserPane};
use crate::{
    types::{project::ProjectStep, update::UpdateStep},
    ui::input,
};
use chrono::Local;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Position, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Text},
    widgets::{
        Block, Cell, Clear, List, ListItem, Paragraph, Row, Scrollbar, ScrollbarState,
        Table, Wrap,
    },
};

const INFO_TEXT: [&str; 1] = ["(Esc) return | (Enter) open | (j/k) row | (h/l) column | (q) quit"];
const UPDATE_ROW_HEIGHT: u16 = 4;

impl App {
    pub(super) fn render(&mut self, frame: &mut Frame, text_in: &mut input::Input) {
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
        let layout = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(1),
        ]);
        let [input_area, help_area, error_area] = frame.area().layout(&layout);

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
            let message = Paragraph::new(error.to_string()).style(Style::default().fg(Color::Red));
            frame.render_widget(message, error_area);
        }

        frame.render_widget(input, input_area);
        #[expect(clippy::cast_possible_truncation)]
        frame.set_cursor_position(Position::new(
            input_area.x + text_in.character_index as u16 + 1,
            input_area.y + 1,
        ));
    }

    fn render_update_input(&self, frame: &mut Frame, text_in: &input::Input) {
        let layout = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(1),
        ]);
        let [input_area, help_area, error_area] = frame.area().layout(&layout);

        let (msg, style) = match text_in.update_step {
            UpdateStep::Title => (
                vec![
                    "Press ".into(),
                    "Esc".bold(),
                    " to stop editing, ".into(),
                    "Enter".bold(),
                    " your update title:".into(),
                ],
                Style::default(),
            ),
            UpdateStep::Body => (
                vec![
                    "Press ".into(),
                    "Esc".bold(),
                    " to stop editing, ".into(),
                    "Enter".bold(),
                    " your update body:".into(),
                ],
                Style::default(),
            ),
            UpdateStep::Next => (
                vec![
                    "Press ".into(),
                    "Esc".bold(),
                    " to stop editing, ".into(),
                    "Enter".bold(),
                    " your next plans:".into(),
                ],
                Style::default(),
            ),
            UpdateStep::Confirm => unreachable!("Confirm is rendered separately"),
        };
        let text = Text::from(Line::from(msg)).patch_style(style);
        let help_message = Paragraph::new(text);
        frame.render_widget(help_message, help_area);

        let input = Paragraph::new(text_in.input.as_str())
            .style(Style::default().fg(Color::Yellow))
            .block(Block::bordered().title(match text_in.update_step {
                UpdateStep::Title => "Update Title",
                UpdateStep::Body => "Update Body",
                UpdateStep::Next => "What's Next?",
                UpdateStep::Confirm => unreachable!("Confirm is rendered separately"),
            }));

        if let Some(error) = &self.err {
            let message = Paragraph::new(error.to_string()).style(Style::default().fg(Color::Red));
            frame.render_widget(message, error_area);
        }

        frame.render_widget(input, input_area);
        #[expect(clippy::cast_possible_truncation)]
        frame.set_cursor_position(Position::new(
            input_area.x + text_in.character_index as u16 + 1,
            input_area.y + 1,
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
            let message = Paragraph::new(error.to_string()).style(Style::default().fg(Color::Red));
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
            " to confirm update.".into(),
        ];

        if let Some(error) = &self.err {
            let message = Paragraph::new(error.to_string()).style(Style::default().fg(Color::Red));
            frame.render_widget(message, error_area);
        }

        let title = text_in.update.title.as_deref().unwrap_or("(missing)");
        let body = text_in.update.body.as_deref().unwrap_or("(missing)");
        let next = text_in.update.next.as_deref().unwrap_or("(missing)");

        let title_msg = vec!["Title: ".into(), title.into()];
        let body_msg = vec!["Body: ".into(), body.into()];
        let next_msg = vec!["Next: ".into(), next.into()];

        let help_text = Text::from(Line::from(help_msg)).patch_style(Style::default());
        let help_message = Paragraph::new(help_text);
        let details = Paragraph::new(Text::from(vec![
            Line::from(title_msg),
            Line::from(body_msg),
            Line::from(next_msg),
        ]))
        .wrap(Wrap { trim: false });

        frame.render_widget(help_message, help_area);
        frame.render_widget(details, details_area);
    }

    fn render_empty_state(&self, frame: &mut Frame) {
        let error_height = if self.err.is_some() { 1 } else { 0 };

        let layout = Layout::vertical([Constraint::Length(error_height), Constraint::Length(3)]);
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
        let help_message = Paragraph::new(text).block(Block::bordered().title("Get Started!"));
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
            let message = Paragraph::new(error.as_str()).style(Style::default().fg(Color::Red));

            frame.render_widget(message, error_area);
        }

        let [project_list_area, latest_update_area] = content_area.layout(&Layout::horizontal([
            Constraint::Percentage(20),
            Constraint::Percentage(80),
        ]));

        let projects: Vec<ListItem<'_>> = self
            .projects
            .iter()
            .map(|project| ListItem::new(project.name.as_str()))
            .collect();

        let projects_border_colour = if self.focused_pane == BrowserPane::Projects {
            Color::Yellow
        } else {
            Color::Reset
        };

        let list = List::new(projects).highlight_symbol("> ").block(
            Block::bordered()
                .title("Projects")
                .border_style(Style::default().fg(projects_border_colour)),
        );
        frame.render_stateful_widget(list, project_list_area, &mut self.project_selection);

        let opened_update = self
            .updates
            .iter()
            .find(|update| {
                self.opened_project_id.as_deref() == Some(update.project_id.as_str())
                    && self.opened_update_id.as_deref() == Some(update.id.as_str())
            })
            .or_else(|| {
                self.updates.iter().find(|update| {
                    self.opened_project_id.as_deref() == Some(update.project_id.as_str())
                })
            });

        if let Some(update) = opened_update {
            let title_msg = vec!["Title: ".into(), update.title.as_str().into()];
            let body_msg = vec!["Body: ".into(), update.body.as_str().into()];
            let next_msg = vec!["What to do next: ".into(), update.next.as_str().into()];

            let created_at_local_time = update.created_at.with_timezone(&Local);
            let updated_at_local_time = update.updated_at.with_timezone(&Local);

            let created_at_display = created_at_local_time.format("%d %b %Y, %H:%M").to_string();
            let updated_at_display = updated_at_local_time.format("%d %b %Y, %H:%M").to_string();

            let created_at_msg = vec!["Created at: ".into(), created_at_display.bold()];
            let updated_at_msg = vec!["Updated at: ".into(), updated_at_display.bold()];

            let text = Text::from(vec![
                Line::from(title_msg),
                Line::from(body_msg),
                Line::from(next_msg),
                Line::from(created_at_msg),
                Line::from(updated_at_msg),
            ]);

            let latest_update_border_colour = if self.focused_pane == BrowserPane::LatestUpdate {
                Color::Yellow
            } else {
                Color::Reset
            };

            let details = Paragraph::new(text).wrap(Wrap { trim: false }).block(
                Block::bordered()
                    .title(update.title.as_str())
                    .border_style(Style::default().fg(latest_update_border_colour)),
            );

            frame.render_widget(details, latest_update_area);
        } else {
            if self.updates.is_empty() {
                let (msg, style) = (
                    vec!["Select a project and press ".into(), "Enter".bold()],
                    Style::default(),
                );
                let text = Text::from(Line::from(msg)).patch_style(style);
                let project_message = Paragraph::new(text).block(Block::bordered());
                frame.render_widget(project_message, latest_update_area);
            } else {
                let (msg, style) = (
                    vec![
                        "There are currently no updates for this project, Press ".into(),
                        "a".bold(),
                        " to create one!".into(),
                    ],
                    Style::default(),
                );

                let text = Text::from(Line::from(msg)).patch_style(style);
                let project_message = Paragraph::new(text)
                    .wrap(Wrap { trim: false })
                    .block(Block::bordered().title("Latest Update"));
                frame.render_widget(project_message, latest_update_area);
            }
        }

        let (msg, style) = (
            vec![
                "(".into(),
                "A".bold(),
                ") New project | (".into(),
                "j/k".bold(),
                ") Down/Up | (".into(),
                "Enter".bold(),
                ") Open project | (".into(),
                "CTRL + h/l".bold(),
                ") Switch panes | (".into(),
                "D".bold(),
                ") Delete project | (".into(),
                "?".bold(),
                ") Help | (".into(),
                "q".bold(),
                ") Quit".into(),
            ],
            Style::default(),
        );
        let text = Text::from(Line::from(msg)).patch_style(style);
        let help_message = Paragraph::new(text);
        frame.render_widget(help_message, help_area);
    }

    fn render_delete_confirmation(&self, frame: &mut Frame) {
        let Some(project) = self
            .projects
            .iter()
            .find(|project| self.pending_project_delete_id.as_deref() == Some(project.id.as_str()))
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
                ": delete    ".into(),
                "Esc".bold(),
                ": cancel".into(),
            ]),
        ])
        .block(Block::bordered().title("Delete project?"));

        frame.render_widget(Clear, popup);
        frame.render_widget(message, popup);
    }

    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        if self.updates.is_empty() {
            frame.render_widget(
                Paragraph::new("No updates are available for this project."),
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

        let header_style = Style::default();
        let selected_row_style = Style::default().add_modifier(Modifier::REVERSED);

        let header = ["Title", "Body", "Next", "Created At", "Updated At"]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(header_style)
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
        .row_highlight_style(selected_row_style)
        .cell_highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
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
                .begin_symbol(None)
                .end_symbol(None),
            scrollbar_area,
            &mut scroll_state,
        );
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let info_footer = Paragraph::new(Text::from_iter(INFO_TEXT))
            .style(Style::new())
            .centered()
            .block(Block::bordered().border_style(Style::new()));

        frame.render_widget(info_footer, area);
    }

    fn render_help_window(&self, frame: &mut Frame) {
        let help = Paragraph::new(vec![
            Line::from("Browser".yellow().bold()),
            Line::from("A: new project"),
            Line::from("?: help"),
            Line::from("q: quit"),
            Line::from("j / Down, k / Up: select a project (Projects focused)"),
            Line::from("Enter: open selected project (Projects focused)"),
            Line::from("Ctrl+h: focus Projects"),
            Line::from("Ctrl+l: focus Latest Update"),
            Line::from("D: delete selected project (Projects focused)"),
            Line::from("a: new update"),
            Line::from("u: update table (requires an open project)"),
            Line::from(""),
            Line::from("Update table".yellow().bold()),
            Line::from("j / Down, k / Up: select row"),
            Line::from("h / Left, l / Right: select column"),
            Line::from("Enter: open selected update"),
            Line::from("Esc: return"),
            Line::from("q: quit"),
            Line::from(""),
            Line::from("Project and update forms".yellow().bold()),
            Line::from("Type: insert text"),
            Line::from("Left / Right: move cursor"),
            Line::from("Backspace: delete previous character"),
            Line::from("Enter: submit field"),
            Line::from("Esc: cancel"),
            Line::from(""),
            Line::from("Delete confirmation".yellow().bold()),
            Line::from("Enter: delete project from Trail"),
            Line::from("Esc: cancel"),
            Line::from(""),
            Line::from("Help".yellow().bold()),
            Line::from("Esc / ?: return"),
            Line::from("q: quit"),
        ])
        .block(Block::bordered().title("Trail keybindings"))
        .wrap(Wrap { trim: false });

        frame.render_widget(help, frame.area());
    }
}
