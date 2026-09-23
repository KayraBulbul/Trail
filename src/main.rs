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
    if args.len() > 1 && (args[1] == "--version" || args[1] == "-v") {
        println!("Trail Version: {}", env!("CARGO_PKG_VERSION"));

        return Ok(());
    }

    let conn = database::sqlite::create_database()?;
    let mut terminal = ratatui::init();

    let mut app = ui::app::App {
        show_project_input: false,
        show_update_input: false,
        show_update_table: false,
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
    Ok(())
}
