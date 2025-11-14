mod application;
// mod command;
mod editor;
mod error;
mod event;
mod logging;
mod outcome;
mod persist;
mod router;
mod workspace;
mod commands;

use crate::application::Application;
use crate::error::AppResult;

fn main() -> AppResult<()> {
    let mut app = Application::new()?;
    if let Err(e) = app.run() {
        e.report();
    }
    Ok(())
}