mod database;
mod types;
mod ui;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let conn = database::sqlite::create_database()?;
    let mut terminal = ratatui::init();

    let mut app = ui::app::App {
        show_project_input: false,
        err: None,
        exit: false,
    };

    let app_result = app.run(&mut terminal, &conn);

    ratatui::restore();
    app_result?;
    Ok(())
}
