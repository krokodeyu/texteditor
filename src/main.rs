mod commands;
mod application;
mod editor_instance;
mod error;
mod text_editor;
mod xml_editor;
mod event;
mod logging;
mod outcome;
mod persist;
mod router;
mod timer;
mod workspace;

use crate::application::Application;
use crate::error::AppResult;

fn main() -> AppResult<()> {
    let mut app = Application::new()?;
    if let Err(e) = app.run() {
        e.report();
    }
    Ok(())
}