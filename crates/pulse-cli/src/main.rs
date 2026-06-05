#![allow(dead_code, clippy::collapsible_if, clippy::collapsible_match)]

mod action;
mod app;
mod client;
mod config;
mod event;
mod github;
mod installer;
mod screens;
mod ssh_config;
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
use screens::setup::SetupScreen;
use std::io::stdout;

#[tokio::main]
async fn main() -> Result<()> {
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
        default_panic(info);
    }));

    let mut config = Config::load().unwrap_or_default();

    // CLI arg: pulse http://master:8080 → quick connect (skip setup)
    if let Some(url) = std::env::args().nth(1) {
        let token = std::env::var("PULSE_ADMIN_TOKEN").ok();
        let name = url.replace("http://", "").replace("https://", "").replace(':', "-").replace('/', "");
        config.add_master(name.clone(), url, token);
        config.current_context = name;
        config.save().ok();
    }

    // Terminal setup
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    // First-run setup?
    if config.is_first_run() && std::env::args().nth(1).is_none() {
        run_setup(&mut terminal, &mut config).await?;
    }

    let mut app = App::new(config);

    // If has current context, go to fleet
    if !app.config.current_context.is_empty() && app.config.current_master().is_some() {
        let name = app.config.current_context.clone();
        app.process_action(action::Action::NavigateFleet(name));
        app.refresh_data().await;
    }

    let result = run(&mut terminal, &mut app).await;

    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    result
}

async fn run_setup(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>, config: &mut Config) -> Result<()> {
    let mut setup = SetupScreen::new();
    let mut events = EventHandler::new(50, 60);
    let theme = ui::theme::Theme::default();

    loop {
        terminal.draw(|frame| setup.render(frame, frame.area(), &theme))?;

        let Some(event) = events.next().await else { break };
        if let Event::Key(key) = event {
            let action = setup.handle_key(key);
            if setup.done {
                if !setup.token_input.is_empty() {
                    config.github_token = Some(setup.token_input.clone());
                    config.github_repo = Some(setup.repo_input.clone());
                    config.save().ok();
                }
                break;
            }
            if matches!(action, action::Action::CloseModal) {
                break;
            }
        }
    }
    Ok(())
}

async fn run(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>, app: &mut App) -> Result<()> {
    let refresh_secs = app.config.preferences.refresh_rate_secs;
    let mut events = EventHandler::new(50, refresh_secs);

    loop {
        app.poll_provision();
        terminal.draw(|frame| app.render(frame))?;

        let Some(event) = events.next().await else { break };

        match event {
            Event::Key(key) => {
                if is_quit(&key)
                    && app.modal.is_none()
                    && !matches!(app.screen, Screen::Fleet if app.fleet.filtering)
                    && !matches!(app.screen, Screen::Exec)
                    && !matches!(app.screen, Screen::Provision)
                {
                    app.quit = true;
                }
                if app.quit {
                    break;
                }

                let action = app.handle_key(key);

                // Handle modal submission
                if matches!(action, action::Action::CloseModal) {
                    if let Some(ref modal) = app.modal
                        && modal.confirmed
                    {
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
                                let name = name.clone();
                                app.config.remove_master(&name);
                                app.config.save().ok();
                            }
                            action::ModalAction::RenameMaster(old_name) => {
                                if let Some(new_name) = modal.fields.first().map(|f| f.value.clone()) {
                                    if !new_name.is_empty() {
                                        if let Some(m) = app.config.masters.iter_mut().find(|m| m.name == *old_name) {
                                            m.name = new_name.clone();
                                        }
                                        if app.config.current_context == *old_name {
                                            app.config.current_context = new_name;
                                        }
                                        app.config.save().ok();
                                    }
                                }
                            }
                            action::ModalAction::RenameAgent(agent_id, _) => {
                                if let Some(new_name) = modal.fields.first().map(|f| f.value.clone()) {
                                    if !new_name.is_empty() {
                                        if let Some(ref client) = app.client {
                                            let url = format!("{}/api/agents/{}/name", client.base_url, agent_id);
                                            let mut req = client.http.put(&url);
                                            if let Some(ref t) = client.token {
                                                req = req.bearer_auth(t);
                                            }
                                            match req.json(&serde_json::json!({"name": new_name})).send().await {
                                                Ok(resp) if resp.status().is_success() => {
                                                    // Force refresh to show new name
                                                    app.refresh_data().await;
                                                }
                                                Ok(resp) => {
                                                    let status = resp.status();
                                                    app.fleet.log_error(format!("Rename falhou: HTTP {status}. Master precisa ser atualizado (v0.3.1+)."));
                                                }
                                                Err(e) => {
                                                    app.fleet.log_error(format!("Rename falhou: {e}"));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    app.modal = None;
                    continue;
                }

                if matches!(action, action::Action::RunExec) {
                    if let Some(ref client) = app.client {
                        app.exec.run_command(client).await;
                    }
                    continue;
                }

                app.process_action(action);

                match app.screen {
                    Screen::Fleet | Screen::Detail | Screen::Update if app.client.is_some() => {
                        app.refresh_data().await;
                    }
                    _ => {}
                }
            }
            Event::Tick => {}
            Event::Refresh => app.refresh_data().await,
        }
    }
    Ok(())
}
