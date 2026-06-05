use crate::action::{Action, ModalAction};
use crate::client::Client;
use crate::config::Config;
use crate::screens::{
    detail::DetailScreen,
    exec::ExecScreen,
    fleet::FleetScreen,
    home::HomeScreen,
    provision::{ProvisionScreen, ProvisionTarget},
    update::UpdateScreen,
};
use crate::ui::theme::Theme;
use crossterm::event::KeyEvent;
use ratatui::Frame;

pub enum Screen {
    Home,
    Fleet,
    Detail,
    Exec,
    Update,
    Provision,
}

pub struct App {
    pub config: Config,
    pub theme: Theme,
    pub screen: Screen,
    pub quit: bool,

    pub home: HomeScreen,
    pub fleet: FleetScreen,
    pub detail: DetailScreen,
    pub exec: ExecScreen,
    pub update: UpdateScreen,
    pub provision: ProvisionScreen,

    pub client: Option<Client>,
    pub modal: Option<Modal>,
}

pub struct Modal {
    pub title: String,
    pub fields: Vec<ModalField>,
    pub focused: usize,
    pub action: ModalAction,
    pub confirmed: bool,
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
            exec: ExecScreen::new(String::new(), String::new()),
            update: UpdateScreen::new(),
            provision: ProvisionScreen::new(ProvisionTarget::Master, String::new(), String::new()),
            client: None,
            modal: None,
            config,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        if let Some(ref mut modal) = self.modal {
            return handle_modal_key(modal, key);
        }

        match self.screen {
            Screen::Home => self.home.handle_key(key, &self.config),
            Screen::Fleet => self.fleet.handle_key(key),
            Screen::Detail => {
                let action = self.detail.handle_key(key);
                // 'e' in detail → exec
                if key.code == crossterm::event::KeyCode::Char('e')
                    && let Some(agent) = self.detail.agent.as_ref()
                {
                    return Action::NavigateExec(agent.id.clone(), agent.hostname.clone());
                }
                action
            }
            Screen::Exec => {
                let action = self.exec.handle_key(key);
                if key.code == crossterm::event::KeyCode::Enter && !self.exec.input.is_empty() && !self.exec.running {
                    return Action::RunExec;
                }
                action
            }
            Screen::Update => self.update.handle_key(key),
            Screen::Provision => self.provision.handle_key(key),
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
                    let token = master.admin_token.clone().or_else(|| std::env::var("PULSE_ADMIN_TOKEN").ok());
                    self.client = Some(Client::new(&master.url, token));
                    self.fleet = FleetScreen::new(name.clone());
                    self.config.current_context = name;
                    self.config.save().ok();
                    self.screen = Screen::Fleet;
                }
            }
            Action::NavigateDetail(agent_id) => {
                self.detail = DetailScreen::new(agent_id.clone());
                self.detail.agent = self.fleet.agents.iter().find(|a| a.id == agent_id).cloned();
                self.screen = Screen::Detail;
            }
            Action::NavigateExec(agent_id, hostname) => {
                self.exec = ExecScreen::new(agent_id, hostname);
                self.screen = Screen::Exec;
            }
            Action::NavigateUpdate => {
                self.screen = Screen::Update;
            }
            Action::NavigateProvision(target) => {
                let master_url = self.config.current_master().map(|m| m.url.clone()).unwrap_or_default();
                // For agent provisioning, try to fetch register token from master
                let register_token = if matches!(target, ProvisionTarget::Agent) {
                    if self.client.is_some() {
                        // Will be fetched async — for now use empty, provision screen will handle
                        String::new()
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };
                self.provision = ProvisionScreen::new(target, master_url, register_token);
                self.screen = Screen::Provision;
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
            Screen::Exec => self.exec.render(frame, area, &self.theme),
            Screen::Update => self.update.render(frame, area, &self.theme),
            Screen::Provision => self.provision.render(frame, area, &self.theme),
        }

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
            Screen::Detail => self.detail.refresh(client).await,
            Screen::Update => self.update.refresh(client).await,
            _ => {}
        }
    }

    pub fn poll_provision(&mut self) {
        if matches!(self.screen, Screen::Provision) {
            self.provision.poll_ssh();
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
                ModalField { label: "Token".into(), value: String::new(), placeholder: "(opcional)".into() },
            ],
            focused: 0,
            action,
            confirmed: false,
        },
        ModalAction::ConfirmDelete(ref name) => {
            Modal { title: format!("Remover '{name}'?"), fields: vec![], focused: 0, action, confirmed: false }
        }
    }
}

fn handle_modal_key(modal: &mut Modal, key: KeyEvent) -> Action {
    use crossterm::event::KeyCode;
    match key.code {
        KeyCode::Esc => Action::CloseModal,
        KeyCode::Tab => {
            if !modal.fields.is_empty() {
                modal.focused = (modal.focused + 1) % modal.fields.len();
            }
            Action::None
        }
        KeyCode::BackTab => {
            if !modal.fields.is_empty() {
                modal.focused = modal.focused.checked_sub(1).unwrap_or(modal.fields.len() - 1);
            }
            Action::None
        }
        KeyCode::Backspace => {
            if let Some(f) = modal.fields.get_mut(modal.focused) {
                f.value.pop();
            }
            Action::None
        }
        KeyCode::Char(c) => {
            if let Some(f) = modal.fields.get_mut(modal.focused) {
                f.value.push(c);
            }
            Action::None
        }
        KeyCode::Enter => {
            if modal.fields.is_empty() || modal.fields.iter().take(2).all(|f| !f.value.is_empty()) {
                modal.confirmed = true;
                Action::CloseModal
            } else {
                if !modal.fields.is_empty() {
                    modal.focused = (modal.focused + 1) % modal.fields.len();
                }
                Action::None
            }
        }
        _ => Action::None,
    }
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

    let mut lines = vec![Line::from("")];

    if modal.fields.is_empty() {
        lines.push(Line::from(Span::styled("  Enter confirma, Esc cancela", theme.normal())));
    } else {
        for (i, field) in modal.fields.iter().enumerate() {
            let focused = i == modal.focused;
            let ls = if focused { theme.title() } else { theme.muted() };
            lines.push(Line::from(Span::styled(format!("  {}:", field.label), ls)));
            let val = if field.value.is_empty() {
                Span::styled(&field.placeholder, theme.muted())
            } else {
                Span::styled(&field.value, theme.normal())
            };
            let cur = if focused { "█" } else { "" };
            lines.push(Line::from(vec![Span::raw("  "), val, Span::styled(cur, theme.normal())]));
            lines.push(Line::from(""));
        }
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.blue))
        .title(format!(" {} ", modal.title));
    frame.render_widget(Paragraph::new(lines).block(block), area);
}
