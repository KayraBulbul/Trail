use ratatui::widgets::ListState;

use crate::ui::app::BrowserPane;

mod database;
mod types;
mod ui;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let conn = database::sqlite::create_database()?;
    let mut terminal = ratatui::init();

    let mut app = ui::app::App {
        show_project_input: false,
        show_update_input: false,
        projects: Vec::new(),
        updates: Vec::new(),
        project_selection: ListState::default(),
        update_selection: ListState::default(),
        opened_project_id: None,
        opened_update_id: None,
        pending_project_delete_id: None,
        focused_pane: BrowserPane::Projects,
        err: None,
        exit: false,
    };

    let app_result = app.run(&mut terminal, &conn);

    ratatui::restore();
    app_result?;
    Ok(())
}
