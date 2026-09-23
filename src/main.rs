use std::env;

use crossterm::{
    event::{KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags},
    execute,
};
use ratatui::widgets::{ListState, TableState};

use crate::ui::app::BrowserPane;

mod database;
mod types;
mod ui;
mod update;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("--version" | "-v") => {
            println!("Trail Version: {}", env!("CARGO_PKG_VERSION"));
            if let Ok(Some(latest)) = update::updater::cached_update() {
                println!("Update available: v{latest}. Run `trail update` to install it.");
            }
            return Ok(());
        }
        Some("update") => {
            match update::updater::check_update()
                .map_err(|error| format!("Couldn't check for updates: {error}"))?
            {
                Some(release) => update::updater::install_update(release)?,
                None => println!("Trail is up to date (v{}).", env!("CARGO_PKG_VERSION")),
            }
            return Ok(());
        }
        Some("--help" | "-h" | "help") => {
            println!("Trail {}", env!("CARGO_PKG_VERSION"));
            println!("Trail allows devs to organise their projects and leave themselves notes/updates so they can pick up where they left off.");
            println!("");
            println!("USAGE:");
            println!("    trail [options] <command>");
            println!("");
            println!("COMMANDS:");
            println!("    update        Install latest version.");
            println!("    help          Print this message.");
            println!("");
            println!("OPTIONS:");
            println!("    -v, --version Print version information.");
            println!("    -h, --help    Print help information.");
            return Ok(());
        }
        _ => {}
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
    let app_result = app.run(&mut terminal, &conn);

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
    Ok(())
}
