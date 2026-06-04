mod app;
mod client;
mod event;
mod ui;

use anyhow::Result;
use app::{Action, App, Screen};
use client::Client;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use event::{Event, EventHandler, is_quit};
use ratatui::prelude::*;
use std::io::stdout;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // Restore terminal on panic
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
        default_panic(info);
    }));

    let master_url = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("PULSE_MASTER_URL").ok())
        .unwrap_or_else(|| "http://localhost:8080".into());

    let token = std::env::var("PULSE_ADMIN_TOKEN").ok();

    let client = Client::new(&master_url, token);
    let mut app = App::new(client);

    // Initial data fetch
    app.refresh_fleet().await;

    // Terminal setup
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;

    let result = run(&mut terminal, &mut app).await;

    // Cleanup
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;

    result
}

async fn run(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>, app: &mut App) -> Result<()> {
    let mut events = EventHandler::new(Duration::from_millis(100), Duration::from_secs(5));

    loop {
        terminal.draw(|frame| ui::render(frame, app))?;

        let Some(event) = events.next().await else { break };

        match event {
            Event::Key(key) => {
                if is_quit(&key) && !app.filtering && app.screen != Screen::Exec {
                    app.quit = true;
                }
                if app.quit {
                    break;
                }
                match app.handle_key(key) {
                    Action::RefreshDetail => app.refresh_detail().await,
                    Action::RefreshRollout => app.refresh_rollout().await,
                    Action::RunExec => app.run_exec().await,
                    Action::None => {}
                }
            }
            Event::Tick => {}
            Event::DataRefresh => match app.screen {
                Screen::Fleet | Screen::Update => app.refresh_fleet().await,
                Screen::Detail | Screen::Exec => app.refresh_detail().await,
            },
        }
    }

    Ok(())
}
