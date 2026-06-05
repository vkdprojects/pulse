use crate::action::{Action, ModalAction};
use crate::client::Client;
use crate::config::Config;
use crate::screens::{detail::DetailScreen, fleet::FleetScreen, home::HomeScreen};
use crate::ui::theme::Theme;
use crossterm::event::KeyEvent;
use ratatui::Frame;

pub enum Screen {
    Home,
    Fleet,
    Detail,
}

pub struct App {
    pub config: Config,
    pub theme: Theme,
    pub screen: Screen,
    pub quit: bool,

    pub home: HomeScreen,
    pub fleet: FleetScreen,
    pub detail: DetailScreen,

    pub client: Option<Client>,
    pub modal: Option<Modal>,
}

pub struct Modal {
    pub title: String,
    pub fields: Vec<ModalField>,
    pub focused: usize,
    pub action: ModalAction,
}

pub struct ModalField {
    pub label: String,
    pub value: String,
    pub placeholder: String,
}

impl App {
    pub fn new(config: Config) -> Self {
        Self {
            theme: Theme::default(),
            screen: Screen::Home,
            quit: false,
            home: HomeScreen::new(),
            fleet: FleetScreen::new(String::new()),
            detail: DetailScreen::new(String::new()),
            client: None,
            modal: None,
            config,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        // Modal captures all input
        if let Some(ref mut modal) = self.modal {
            return handle_modal_key(modal, key);
        }

        match self.screen {
            Screen::Home => self.home.handle_key(key, &self.config),
            Screen::Fleet => self.fleet.handle_key(key),
            Screen::Detail => self.detail.handle_key(key),
        }
    }

    pub fn process_action(&mut self, action: Action) {
        match action {
            Action::Quit => self.quit = true,
            Action::NavigateHome => {
                self.screen = Screen::Home;
                self.client = None;
            }
            Action::NavigateFleet(name) => {
                let name = if name.is_empty() { self.fleet.context_name.clone() } else { name };
                if let Some(master) = self.config.masters.iter().find(|m| m.name == name) {
                    self.client = Some(Client::new(&master.url, master.admin_token.clone()));
                    self.fleet = FleetScreen::new(name.clone());
                    self.config.current_context = name;
                    self.config.save().ok();
                    self.screen = Screen::Fleet;
                }
            }
            Action::NavigateDetail(agent_id) => {
                self.detail = DetailScreen::new(agent_id.clone());
                // Set agent info from fleet data
                self.detail.agent = self.fleet.agents.iter().find(|a| a.id == agent_id).cloned();
                self.screen = Screen::Detail;
            }
            Action::ShowModal(modal_action) => {
                self.modal = Some(create_modal(modal_action));
            }
            Action::CloseModal => {
                self.modal = None;
            }
            Action::DataLoaded(agents) => {
                let mut new_list = Vec::with_capacity(agents.len());
                for new_agent in agents {
                    let mut agent = new_agent;
                    // Preserve sparkline history from existing data
                    if let Some(existing) = self.fleet.agents.iter().find(|a| a.id == agent.id) {
                        agent.cpu_history = existing.cpu_history.clone();
                    }
                    if let Some(cpu) = agent.cpu_pct {
                        agent.cpu_history.push(cpu as u64);
                        if agent.cpu_history.len() > 60 {
                            agent.cpu_history.remove(0);
                        }
                    }
                    new_list.push(agent);
                }
                self.fleet.agents = new_list;
            }
            _ => {}
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        let area = frame.area();
        match self.screen {
            Screen::Home => self.home.render(frame, area, &self.config, &self.theme),
            Screen::Fleet => self.fleet.render(frame, area, &self.theme),
            Screen::Detail => self.detail.render(frame, area, &self.theme),
        }

        // Render modal on top
        if let Some(ref modal) = self.modal {
            render_modal(frame, modal, &self.theme);
        }
    }

    pub async fn refresh_data(&mut self) {
        let Some(ref client) = self.client else { return };
        match self.screen {
            Screen::Fleet => {
                if let Ok(agents) = client.overview().await {
                    self.process_action(Action::DataLoaded(agents));
                }
            }
            Screen::Detail => {
                self.detail.refresh(client).await;
            }
            _ => {}
        }
    }
}

fn create_modal(action: ModalAction) -> Modal {
    match action {
        ModalAction::AddMaster => Modal {
            title: "Adicionar Master".into(),
            fields: vec![
                ModalField { label: "Nome".into(), value: String::new(), placeholder: "production".into() },
                ModalField { label: "URL".into(), value: String::new(), placeholder: "http://master:8080".into() },
                ModalField { label: "Token (opcional)".into(), value: String::new(), placeholder: "".into() },
            ],
            focused: 0,
            action,
        },
        ModalAction::ConfirmDelete(ref name) => {
            Modal { title: format!("Remover '{name}'?"), fields: vec![], focused: 0, action }
        }
    }
}

fn handle_modal_key(modal: &mut Modal, key: KeyEvent) -> Action {
    use crossterm::event::KeyCode;
    match key.code {
        KeyCode::Esc => return Action::CloseModal,
        KeyCode::Tab => {
            if !modal.fields.is_empty() {
                modal.focused = (modal.focused + 1) % modal.fields.len();
            }
        }
        KeyCode::BackTab => {
            if !modal.fields.is_empty() {
                modal.focused = modal.focused.checked_sub(1).unwrap_or(modal.fields.len() - 1);
            }
        }
        KeyCode::Backspace => {
            if let Some(field) = modal.fields.get_mut(modal.focused) {
                field.value.pop();
            }
        }
        KeyCode::Char(c) => {
            if let Some(field) = modal.fields.get_mut(modal.focused) {
                field.value.push(c);
            }
        }
        KeyCode::Enter => {
            if modal.fields.is_empty() {
                // Confirm action (delete)
                return Action::CloseModal; // caller handles via modal.action
            }
            // Check if we can submit
            if modal.fields.iter().take(2).all(|f| !f.value.is_empty()) {
                return Action::CloseModal; // caller extracts values
            }
            // Move to next field
            if !modal.fields.is_empty() {
                modal.focused = (modal.focused + 1) % modal.fields.len();
            }
        }
        _ => {}
    }
    Action::None
}

fn render_modal(frame: &mut Frame, modal: &Modal, theme: &Theme) {
    use crate::ui::modal::centered_rect;
    use ratatui::{
        style::Style,
        text::{Line, Span},
        widgets::{Block, Borders, Clear, Paragraph},
    };

    let area = centered_rect(50, 40, frame.area());
    frame.render_widget(Clear, area);

    let mut lines = Vec::new();
    lines.push(Line::from(""));

    if modal.fields.is_empty() {
        // Confirm dialog
        lines.push(Line::from(Span::styled("  Enter para confirmar, Esc para cancelar", theme.normal())));
    } else {
        for (i, field) in modal.fields.iter().enumerate() {
            let focused = i == modal.focused;
            let label_style = if focused { theme.header() } else { theme.muted() };
            lines.push(Line::from(Span::styled(format!("  {}:", field.label), label_style)));

            let val_display = if field.value.is_empty() {
                Span::styled(&field.placeholder, theme.muted())
            } else {
                Span::styled(&field.value, theme.normal())
            };
            let cursor = if focused { "█" } else { "" };
            lines.push(Line::from(vec![Span::raw("  "), val_display, Span::styled(cursor, theme.normal())]));
            lines.push(Line::from(""));
        }
        lines.push(Line::from(Span::styled("  Tab: próximo campo │ Enter: confirmar │ Esc: cancelar", theme.muted())));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.accent))
        .title(format!(" {} ", modal.title));
    let widget = Paragraph::new(lines).block(block);
    frame.render_widget(widget, area);
}
