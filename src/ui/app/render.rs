use super::{App, BrowserPane};
use crate::{
    types::{project::ProjectStep, update::UpdateStep},
    ui::input,
};
use chrono::Local;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Position, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Text},
    widgets::{Block, Clear, List, ListItem, Paragraph, Wrap},
};

impl App {
    pub(super) fn render(&mut self, frame: &mut Frame, text_in: &mut input::Input) {
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
        } else if self.show_update_input
            && (text_in.update_step == UpdateStep::Title
                || text_in.update_step == UpdateStep::Body
                || text_in.update_step == UpdateStep::Next)
        {
            let layout = Layout::vertical([
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Length(1),
            ]);
            let [help_area, input_area, error_area] = frame.area().layout(&layout);

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
        } else if self.show_update_input && text_in.update_step == UpdateStep::Confirm {
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
                let message =
                    Paragraph::new(error.to_string()).style(Style::default().fg(Color::Red));
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
        } else if self.projects.is_empty() {
            let error_height = if self.err.is_some() { 1 } else { 0 };

            let layout =
                Layout::vertical([Constraint::Length(error_height), Constraint::Length(3)]);
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

            let [project_list_area, project_area, update_area] =
                content_area.layout(&Layout::horizontal([
                    Constraint::Percentage(20),
                    Constraint::Percentage(60),
                    Constraint::Percentage(20),
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

                let created_at_display =
                    created_at_local_time.format("%d %b %Y, %H:%M").to_string();
                let updated_at_display =
                    updated_at_local_time.format("%d %b %Y, %H:%M").to_string();

                let created_at_msg = vec!["Created at: ".into(), created_at_display.bold()];
                let updated_at_msg = vec!["Updated at: ".into(), updated_at_display.bold()];

                let text = Text::from(vec![
                    Line::from(title_msg),
                    Line::from(body_msg),
                    Line::from(next_msg),
                    Line::from(created_at_msg),
                    Line::from(updated_at_msg),
                ]);

                let latest_update_border_colour = if self.focused_pane == BrowserPane::LatestUpdate
                {
                    Color::Yellow
                } else {
                    Color::Reset
                };

                let details = Paragraph::new(text).wrap(Wrap { trim: false }).block(
                    Block::bordered()
                        .title(update.title.as_str())
                        .border_style(Style::default().fg(latest_update_border_colour)),
                );

                frame.render_widget(details, project_area);
            } else {
                let (msg, style) = (
                    vec!["Select a project and press ".into(), "Enter".bold()],
                    Style::default(),
                );
                let text = Text::from(Line::from(msg)).patch_style(style);
                let project_message =
                    Paragraph::new(text).block(Block::bordered().title("Select a Project"));
                frame.render_widget(project_message, project_area);
            }

            let updates: Vec<ListItem<'_>> = self
                .updates
                .iter()
                .map(|update| ListItem::new(update.title.as_str()))
                .collect();

            let updates_border_colour = if self.focused_pane == BrowserPane::Updates {
                Color::Yellow
            } else {
                Color::Reset
            };

            let list = List::new(updates).highlight_symbol("> ").block(
                Block::bordered()
                    .title("Updates")
                    .border_style(Style::default().fg(updates_border_colour)),
            );
            frame.render_stateful_widget(list, update_area, &mut self.update_selection);

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
}
