use crate::{
    database::sqlite,
    git::repo::{self, Branch, Commit, Diff, Head, Summary},
    types::{project::Project, update::Update},
    ui::input::Input,
    update::updater::Release,
};

use crossterm::event::Event;
use ratatui::{
    DefaultTerminal,
    widgets::{ListState, TableState},
};
use rusqlite::{Connection, Result};
use std::{
    collections::HashSet,
    io,
    path::{Path, PathBuf},
    sync::mpsc::Receiver,
    time::Duration,
};

mod keybindings;
mod render;

#[cfg(test)]
mod tests;

#[derive(PartialEq)]
pub enum BrowserPane {
    Projects,
    LatestUpdate,
}

#[derive(PartialEq, Default)]
pub enum GitPane {
    #[default]
    List,
    Diff,
}

pub enum GitEntry {
    Uncommitted,
    Commit(Commit),
}

#[derive(PartialEq)]
pub enum DiffSource {
    Uncommitted,
    Commit(String),
}

#[derive(Default)]
pub struct GitState {
    /// Ids of projects whose directory is inside a git repo.
    pub repos: HashSet<String>,
    pub summary: Option<Summary>,
    pub branches: Vec<Branch>,
    pub branch_selection: ListState,
    /// When set, the list shows this branch's commits instead of the branches.
    pub opened_branch: Option<String>,
    pub entries: Vec<GitEntry>,
    pub entry_selection: ListState,
    pub diff: Option<Diff>,
    pub diff_source: Option<DiffSource>,
    pub diff_title: String,
    pub diff_scroll: u16,
    pub focused_pane: GitPane,
}

pub struct App {
    pub update_rx: Receiver<Release>,
    pub pending_release: Option<Release>,
    pub show_project_input: bool,
    pub show_update_input: bool,
    pub show_update_table: bool,
    pub show_update_popup: bool,
    pub show_git_view: bool,
    pub install_on_exit: bool,
    pub show_help: bool,
    pub help_scroll: u16,
    pub detail_scroll: u16,
    pub confirmation_scroll: u16,
    pub projects: Vec<Project>,
    pub updates: Vec<Update>,
    pub update_selection: TableState,
    pub project_selection: ListState,
    pub opened_project_id: Option<String>,
    pub opened_update_id: Option<String>,
    pub pending_project_delete_id: Option<String>,
    pub pending_update_delete_id: Option<String>,
    pub focused_pane: BrowserPane,
    pub git: GitState,
    pub err: Option<String>,
    pub exit: bool,
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal, conn: &Connection) -> io::Result<()> {
        let mut text_in = Input::new();

        if let Err(err) = self.reload_projects(conn) {
            self.err = Some(format!("Couldn't load projects: {err}"));
        }

        while !self.exit {
            if let Ok(release) = self.update_rx.try_recv() {
                self.pending_release = Some(release);
            }
            if self.pending_release.is_some() && self.is_idle() {
                self.show_update_popup = true;
            }

            terminal.draw(|frame| self.render(frame, &mut text_in))?;

            if crossterm::event::poll(Duration::from_millis(250))? {
                if let Event::Key(key_event) = crossterm::event::read()? {
                    self.handle_key_event(key_event, &mut text_in, conn)?;
                }
            }
        }

        Ok(())
    }

    fn is_idle(&self) -> bool {
        !self.show_update_table
            && !self.show_update_input
            && !self.show_project_input
            && !self.show_help
            && !self.show_git_view
            && self.pending_project_delete_id.is_none()
            && self.pending_update_delete_id.is_none()
    }

    fn start_update_edit(&mut self, conn: &Connection, text_in: &mut Input, id: &str) {
        match sqlite::get_update(conn, id) {
            Ok(update) => {
                text_in.edit_update(update);
                self.show_update_input = true;
                self.err = None;
            }
            Err(error) => self.err = Some(format!("Couldn't load update: {error}")),
        }
    }

    fn displayed_update(&self) -> Option<&Update> {
        self.updates
            .iter()
            .find(|update| {
                self.opened_project_id.as_deref() == Some(update.project_id.as_str())
                    && self.opened_update_id.as_deref() == Some(update.id.as_str())
            })
            .or_else(|| {
                self.updates.iter().find(|update| {
                    self.opened_project_id.as_deref() == Some(update.project_id.as_str())
                })
            })
    }

    fn delete_project(&mut self, conn: &Connection, id: &str) {
        if let Err(error) = sqlite::delete_project(conn, id) {
            self.err = Some(format!("Unable to delete project: {error}"));
            return;
        }

        self.err = None;
        if self.opened_project_id.as_deref() == Some(id) {
            self.opened_project_id = None;
            self.git.summary = None;
            self.clear_updates();
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

    fn delete_update(&mut self, conn: &Connection, id: &str) {
        if let Err(error) = sqlite::delete_update(conn, id) {
            self.err = Some(format!("Couldn't delete update: {error}"));
            return;
        }

        self.err = None;
        if self.opened_update_id.as_deref() == Some(id) {
            self.opened_update_id = None;
        }
        let previous_index = self.update_selection.selected().unwrap_or(0);
        if let Err(error) = self.reload_updates(conn) {
            self.err = Some(format!("Couldn't reload updates: {error}"));
        } else if self.updates.is_empty() {
            self.update_selection = TableState::default();
        } else {
            self.update_selection
                .select(Some(previous_index.min(self.updates.len() - 1)));
        }
    }

    fn clear_updates(&mut self) {
        self.detail_scroll = 0;
        self.updates.clear();
        self.update_selection = TableState::default();
        self.opened_update_id = None;
    }

    fn reload_projects(&mut self, conn: &Connection) -> Result<()> {
        let projects = sqlite::get_projects(conn)?;

        let selected = if projects.is_empty() { None } else { Some(0) };

        self.git.repos = projects
            .iter()
            .filter(|project| repo::detect(Path::new(&project.directory)).is_some())
            .map(|project| project.id.clone())
            .collect();
        self.projects = projects;
        self.project_selection.select(selected);

        Ok(())
    }

    fn reload_updates(&mut self, conn: &Connection) -> Result<()> {
        let Some(id) = self.opened_project_id.clone() else {
            unreachable!();
        };

        let updates = sqlite::get_updates(conn, id.as_str())?;

        let selected = if updates.is_empty() { None } else { Some(0) };

        self.detail_scroll = 0;
        self.updates = updates;
        self.update_selection.select(selected);

        Ok(())
    }

    fn opened_git_directory(&self) -> Option<PathBuf> {
        let id = self.opened_project_id.as_deref()?;
        if !self.git.repos.contains(id) {
            return None;
        }
        self.projects
            .iter()
            .find(|project| project.id == id)
            .map(|project| PathBuf::from(&project.directory))
    }

    fn reload_git_summary(&mut self) {
        self.git.summary = None;
        let Some(dir) = self.opened_git_directory() else {
            return;
        };

        match repo::summary(&dir) {
            Ok(summary) => self.git.summary = Some(summary),
            Err(error) => self.err = Some(format!("Couldn't read git status: {error}")),
        }
    }

    fn open_git_view(&mut self) {
        self.git.opened_branch = None;
        self.git.entries.clear();
        self.git.diff = None;
        self.git.diff_source = None;
        self.git.diff_scroll = 0;
        self.git.focused_pane = GitPane::List;
        self.git.branch_selection = ListState::default();
        self.reload_git();

        let head = self.git.branches.iter().position(|branch| branch.is_head);
        self.git
            .branch_selection
            .select(head.or(if self.git.branches.is_empty() {
                None
            } else {
                Some(0)
            }));

        // A repo without commits has no branches to pick, so go straight to its (empty) commits.
        if self.git.branches.is_empty()
            && let Some(Head::Branch(name)) = self.git.summary.as_ref().map(|summary| &summary.head)
        {
            let name = name.clone();
            self.open_git_branch(name);
        }
        self.show_git_view = true;
    }

    fn reload_git(&mut self) {
        let Some(dir) = self.opened_git_directory() else {
            return;
        };
        self.err = None;
        self.reload_git_summary();

        match repo::branches(&dir) {
            Ok(branches) => {
                let selected = self.git.branch_selection.selected();
                self.git
                    .branch_selection
                    .select(match (selected, branches.len()) {
                        (_, 0) => None,
                        (Some(index), len) => Some(index.min(len - 1)),
                        (None, _) => Some(0),
                    });
                self.git.branches = branches;
            }
            Err(error) => self.err = Some(format!("Couldn't load branches: {error}")),
        }

        if let Some(branch) = self.git.opened_branch.clone() {
            let selected = self.git.entry_selection.selected();
            self.open_git_branch(branch);
            if let Some(index) = selected
                && !self.git.entries.is_empty()
            {
                self.git
                    .entry_selection
                    .select(Some(index.min(self.git.entries.len() - 1)));
            }
        }

        if let Some(source) = self.git.diff_source.take() {
            let title = std::mem::take(&mut self.git.diff_title);
            let (scroll, focused_pane) = (
                self.git.diff_scroll,
                std::mem::take(&mut self.git.focused_pane),
            );
            self.open_git_diff(source, title);
            self.git.diff_scroll = scroll;
            self.git.focused_pane = focused_pane;
        }
    }

    fn open_git_branch(&mut self, branch: String) {
        let Some(dir) = self.opened_git_directory() else {
            return;
        };
        let commits = if self.git.branches.is_empty() {
            Ok(Vec::new())
        } else {
            repo::commits(&dir, &branch, repo::COMMIT_LIMIT)
        };

        match commits {
            Ok(commits) => {
                let is_head = self
                    .git
                    .summary
                    .as_ref()
                    .is_some_and(|summary| summary.head == Head::Branch(branch.clone()));
                let dirty = self
                    .git
                    .summary
                    .as_ref()
                    .is_some_and(|summary| summary.files_changed > 0);

                let mut entries = Vec::new();
                if is_head && dirty {
                    entries.push(GitEntry::Uncommitted);
                }
                entries.extend(commits.into_iter().map(GitEntry::Commit));

                let selected = if entries.is_empty() { None } else { Some(0) };
                self.git.entries = entries;
                self.git.entry_selection = ListState::default();
                self.git.entry_selection.select(selected);
                self.git.opened_branch = Some(branch);
            }
            Err(error) => self.err = Some(format!("Couldn't load commits: {error}")),
        }
    }

    fn open_git_diff(&mut self, source: DiffSource, title: String) {
        let Some(dir) = self.opened_git_directory() else {
            return;
        };
        let diff = match &source {
            DiffSource::Uncommitted => repo::uncommitted_diff(&dir),
            DiffSource::Commit(sha) => repo::commit_diff(&dir, sha),
        };

        match diff {
            Ok(diff) => {
                self.git.diff = Some(diff);
                self.git.diff_source = Some(source);
                self.git.diff_title = title;
                self.git.diff_scroll = 0;
                self.git.focused_pane = GitPane::Diff;
            }
            Err(error) => self.err = Some(format!("Couldn't load diff: {error}")),
        }
    }

    fn git_list_selection(&mut self) -> (&mut ListState, usize) {
        if self.git.opened_branch.is_some() {
            (&mut self.git.entry_selection, self.git.entries.len())
        } else {
            (&mut self.git.branch_selection, self.git.branches.len())
        }
    }
}
