#![allow(dead_code)]

mod action;
mod app;
mod client;
mod config;
mod event;
mod screens;
mod ui;

use anyhow::Result;
use app::{App, Screen};
use config::Config;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use event::{Event, EventHandler, is_quit};
use ratatui::prelude::*;
use std::io::stdout;

#[tokio::main]
async fn main() -> Result<()> {
    // Restore terminal on panic
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
        default_panic(info);
    }));

    let mut config = Config::load().unwrap_or_default();

    // CLI arg: pulse http://master:8080 → quick connect (skip home screen)
    if let Some(url) = std::env::args().nth(1) {
        let token = std::env::var("PULSE_ADMIN_TOKEN").ok();
        let name = url.replace("http://", "").replace("https://", "").replace(':', "-").replace('/', "");
        config.add_master(name.clone(), url, token);
        config.current_context = name;
        config.save().ok();
    }

    let mut app = App::new(config);

    // If has current_context, go directly to fleet
    if !app.config.current_context.is_empty() && app.config.current_master().is_some() {
        let name = app.config.current_context.clone();
        app.process_action(action::Action::NavigateFleet(name));
        app.refresh_data().await;
    }

    // Terminal setup
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let result = run(&mut terminal, &mut app).await;

    // Cleanup
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;

    result
}

async fn run(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>, app: &mut App) -> Result<()> {
    let refresh_secs = app.config.preferences.refresh_rate_secs;
    let mut events = EventHandler::new(50, refresh_secs);

    loop {
        terminal.draw(|frame| app.render(frame))?;

        let Some(event) = events.next().await else { break };

        match event {
            Event::Key(key) => {
                // Global quit (only when no modal and not in filter mode)
                if is_quit(&key) && app.modal.is_none() && !matches!(app.screen, Screen::Fleet if app.fleet.filtering) {
                    app.quit = true;
                }
                if app.quit {
                    break;
                }

                let action = app.handle_key(key);

                // Handle modal submission
                if matches!(action, action::Action::CloseModal) {
                    if let Some(ref modal) = app.modal {
                        match &modal.action {
                            action::ModalAction::AddMaster => {
                                if modal.fields.len() >= 2
                                    && !modal.fields[0].value.is_empty()
                                    && !modal.fields[1].value.is_empty()
                                {
                                    let name = modal.fields[0].value.clone();
                                    let url = modal.fields[1].value.clone();
                                    let token = modal.fields.get(2).map(|f| f.value.clone()).filter(|s| !s.is_empty());
                                    app.config.add_master(name, url, token);
                                    app.config.save().ok();
                                }
                            }
                            action::ModalAction::ConfirmDelete(name) => {
                                if key.code == crossterm::event::KeyCode::Enter {
                                    app.config.remove_master(name);
                                    app.config.save().ok();
                                }
                            }
                        }
                    }
                    app.modal = None;
                    continue;
                }

                app.process_action(action);

                // Trigger data fetch on navigation
                if matches!(app.screen, Screen::Fleet | Screen::Detail) && app.client.is_some() {
                    app.refresh_data().await;
                }
            }
            Event::Tick => {}
            Event::Refresh => {
                app.refresh_data().await;
            }
        }
    }

    Ok(())
}
