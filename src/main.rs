use std::process::ExitCode;

use clap::Parser;

use crossterm::{
    event::{
        DisableMouseCapture, EnableMouseCapture, KeyboardEnhancementFlags,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
};
use ratatui::widgets::{ListState, TableState};

use crate::ui::app::{BrowserPane, GitState};

mod cli;
mod core;
mod database;
mod format;
mod git;
mod mcp;
mod types;
mod ui;
mod update;

fn main() -> Result<ExitCode, Box<dyn std::error::Error>> {
    let cli = cli::Cli::parse();
    if cli.version {
        cli::print_version();
        return Ok(ExitCode::SUCCESS);
    }
    if let Some(command) = cli.command {
        return cli::run(command, cli.json);
    }

    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        if let Ok(Some(release)) = update::updater::need_update() {
            let _ = tx.send(release);
        }
    });

    let conn = database::sqlite::create_database()?;
    let mut terminal = ratatui::init();

    let mut app = ui::app::App {
        update_rx: rx,
        pending_release: None,
        show_project_input: false,
        show_update_input: false,
        show_update_table: false,
        show_update_popup: false,
        show_git_view: false,
        install_on_exit: false,
        show_help: false,
        help_scroll: 0,
        detail_scroll: 0,
        confirmation_scroll: 0,
        projects: Vec::new(),
        updates: Vec::new(),
        project_selection: ListState::default(),
        update_selection: TableState::default(),
        opened_project_id: None,
        opened_update_id: None,
        pending_project_delete_id: None,
        pending_update_delete_id: None,
        focused_pane: BrowserPane::Projects,
        git: GitState::default(),
        clicks: Vec::new(),
        update_click: None,
        err: None,
        exit: false,
    };

    let supports_keyboard_enhancement = matches!(
        crossterm::terminal::supports_keyboard_enhancement(),
        Ok(true)
    );

    if supports_keyboard_enhancement {
        execute!(
            std::io::stdout(),
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
        )?;
    }
    execute!(std::io::stdout(), EnableMouseCapture)?;
    let app_result = app.run(&mut terminal, &conn);
    execute!(std::io::stdout(), DisableMouseCapture)?;

    if supports_keyboard_enhancement {
        execute!(std::io::stdout(), PopKeyboardEnhancementFlags)?;
    }

    ratatui::restore();
    app_result?;

    if app.install_on_exit
        && let Some(release) = app.pending_release.take()
    {
        update::updater::install_update(release)?;
    }
    Ok(ExitCode::SUCCESS)
}
