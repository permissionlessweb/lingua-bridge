use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io;

mod app;
mod event;
pub mod gpu;
mod input;
mod screens;
pub mod sdl;
mod theme;
mod ui;
pub mod widgets;

pub mod api;
pub mod config;
pub mod wallet;

pub use app::App;
pub use event::EventHandler;

/// Run the TUI application
pub async fn run_tui() -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app and run it
    let mut app = App::new();
    let res = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    let mut event_handler = EventHandler::new();
    app.set_sender(event_handler.sender());

    loop {
        // Draw the current state
        terminal.draw(|f| ui::render(f, app))?;

        // Handle events
        if let Some(event) = event_handler.next().await {
            if !app.handle_event(event) {
                break; // Exit on quit
            }
        }
    }

    Ok(())
}
