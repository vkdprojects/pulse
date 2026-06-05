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
            provision: ProvisionScreen::new(
                ProvisionTarget::Master,
                String::new(),
                String::new(),
                String::new(),
                String::new(),
            ),
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
                let master = self.config.current_master().cloned();
                let master_url = master.as_ref().map(|m| m.url.clone()).unwrap_or_default();
                let admin_token = master.as_ref().and_then(|m| m.admin_token.clone()).unwrap_or_default();
                let github_token = self.config.github_token.clone().unwrap_or_default();
                let github_repo = self.config.repo().to_string();
                self.provision = ProvisionScreen::new(target, github_token, github_repo, master_url, admin_token);
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
            self.provision.poll_events();
            // If install completed with result, save master config
            if self.provision.done
                && let Some(ref result) = self.provision.result
                && !result.admin_token.is_empty()
            {
                // Master was installed — save to config
                let name = self
                    .provision
                    .fields
                    .iter()
                    .find(|f| f.label == "Nome")
                    .map(|f| f.value.clone())
                    .unwrap_or_else(|| "master".into());
                self.config.add_master(name, result.master_url.clone(), Some(result.admin_token.clone()));
                self.config.save().ok();
            }
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
        ModalAction::RenameMaster(ref name) => Modal {
            title: "Renomear Master".into(),
            fields: vec![ModalField { label: "Novo nome".into(), value: name.clone(), placeholder: "".into() }],
            focused: 0,
            action,
            confirmed: false,
        },
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::AgentOverview;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn app_with_masters() -> App {
        let mut config = Config::default();
        config.add_master("prod".into(), "http://prod:8080".into(), Some("token".into()));
        config.add_master("staging".into(), "http://staging:8080".into(), None);
        App::new(config)
    }

    fn app_in_fleet() -> App {
        let mut app = app_with_masters();
        app.process_action(Action::NavigateFleet("prod".into()));
        app.fleet.agents = vec![
            AgentOverview {
                id: "agent-001".into(),
                hostname: "web-01".into(),
                status: "online".into(),
                cpu_pct: Some(23.0),
                mem_pct: Some(67.0),
                uptime_secs: Some(86400),
                version: "0.3.0".into(),
                cpu_history: vec![10, 20, 30],
            },
            AgentOverview {
                id: "agent-002".into(),
                hostname: "db-01".into(),
                status: "offline".into(),
                cpu_pct: None,
                mem_pct: None,
                uptime_secs: None,
                version: "0.2.0".into(),
                cpu_history: vec![],
            },
        ];
        app
    }

    // ─── Navigation Tests ─────────────────────────────────────────

    #[test]
    fn test_home_enter_navigates_to_fleet() {
        let mut app = app_with_masters();
        assert!(matches!(app.screen, Screen::Home));

        let action = app.handle_key(key(KeyCode::Enter));
        app.process_action(action);

        assert!(matches!(app.screen, Screen::Fleet));
        assert!(app.client.is_some());
        assert_eq!(app.fleet.context_name, "prod");
    }

    #[test]
    fn test_home_n_opens_modal() {
        let mut app = app_with_masters();
        let action = app.handle_key(key(KeyCode::Char('n')));

        assert!(matches!(action, Action::ShowModal(ModalAction::AddMaster)));
        app.process_action(action);
        assert!(app.modal.is_some());
    }

    #[test]
    fn test_home_s_navigates_to_provision() {
        let mut app = app_with_masters();
        let action = app.handle_key(key(KeyCode::Char('s')));
        app.process_action(action);

        assert!(matches!(app.screen, Screen::Provision));
        assert_eq!(app.provision.target, ProvisionTarget::Master);
    }

    #[test]
    fn test_fleet_enter_navigates_to_detail() {
        let mut app = app_in_fleet();
        let action = app.handle_key(key(KeyCode::Enter));
        app.process_action(action);

        assert!(matches!(app.screen, Screen::Detail));
        assert_eq!(app.detail.agent_id, "agent-001");
        assert_eq!(app.detail.agent.as_ref().unwrap().hostname, "web-01");
    }

    #[test]
    fn test_fleet_esc_navigates_to_home() {
        let mut app = app_in_fleet();
        let action = app.handle_key(key(KeyCode::Esc));
        app.process_action(action);

        assert!(matches!(app.screen, Screen::Home));
        assert!(app.client.is_none());
    }

    #[test]
    fn test_fleet_u_navigates_to_update() {
        let mut app = app_in_fleet();
        let action = app.handle_key(key(KeyCode::Char('u')));
        app.process_action(action);

        assert!(matches!(app.screen, Screen::Update));
    }

    #[test]
    fn test_fleet_n_navigates_to_provision_agent() {
        let mut app = app_in_fleet();
        let action = app.handle_key(key(KeyCode::Char('n')));
        app.process_action(action);

        assert!(matches!(app.screen, Screen::Provision));
        assert_eq!(app.provision.target, ProvisionTarget::Agent);
        // 5 fields: host, port, user, auth mode, secret
        assert_eq!(app.provision.fields.len(), 5);
    }

    #[test]
    fn test_detail_e_navigates_to_exec() {
        let mut app = app_in_fleet();
        // Navigate to detail first
        app.process_action(Action::NavigateDetail("agent-001".into()));
        assert!(matches!(app.screen, Screen::Detail));

        let action = app.handle_key(key(KeyCode::Char('e')));
        app.process_action(action);

        assert!(matches!(app.screen, Screen::Exec));
        assert_eq!(app.exec.agent_id, "agent-001");
        assert_eq!(app.exec.hostname, "web-01");
    }

    #[test]
    fn test_detail_esc_navigates_back_to_fleet() {
        let mut app = app_in_fleet();
        app.process_action(Action::NavigateDetail("agent-001".into()));

        let action = app.handle_key(key(KeyCode::Esc));
        app.process_action(action);

        assert!(matches!(app.screen, Screen::Fleet));
    }

    #[test]
    fn test_exec_esc_navigates_back_to_detail() {
        let mut app = app_in_fleet();
        app.process_action(Action::NavigateDetail("agent-001".into()));
        app.process_action(Action::NavigateExec("agent-001".into(), "web-01".into()));
        assert!(matches!(app.screen, Screen::Exec));

        let action = app.handle_key(key(KeyCode::Esc));
        app.process_action(action);

        assert!(matches!(app.screen, Screen::Detail));
    }

    #[test]
    fn test_update_esc_navigates_back_to_fleet() {
        let mut app = app_in_fleet();
        app.process_action(Action::NavigateUpdate);

        let action = app.handle_key(key(KeyCode::Esc));
        app.process_action(action);

        assert!(matches!(app.screen, Screen::Fleet));
    }

    #[test]
    fn test_provision_esc_navigates_back_to_home() {
        let mut app = app_with_masters();
        app.process_action(Action::NavigateProvision(ProvisionTarget::Master));

        let action = app.handle_key(key(KeyCode::Esc));
        app.process_action(action);

        assert!(matches!(app.screen, Screen::Home));
    }

    // ─── Modal Tests ──────────────────────────────────────────────

    #[test]
    fn test_modal_esc_does_not_confirm() {
        let mut app = app_with_masters();
        app.process_action(Action::ShowModal(ModalAction::AddMaster));
        assert!(app.modal.is_some());

        // Press Esc — should close without confirming
        let action = app.handle_key(key(KeyCode::Esc));
        assert!(matches!(action, Action::CloseModal));
        assert!(!app.modal.as_ref().unwrap().confirmed);
    }

    #[test]
    fn test_modal_enter_with_data_confirms() {
        let mut app = app_with_masters();
        app.process_action(Action::ShowModal(ModalAction::AddMaster));

        // Fill fields
        let modal = app.modal.as_mut().unwrap();
        modal.fields[0].value = "new-master".into();
        modal.fields[1].value = "http://new:8080".into();

        // Press Enter — should confirm
        let action = app.handle_key(key(KeyCode::Enter));
        assert!(matches!(action, Action::CloseModal));
        assert!(app.modal.as_ref().unwrap().confirmed);
    }

    #[test]
    fn test_modal_enter_without_data_does_not_confirm() {
        let mut app = app_with_masters();
        app.process_action(Action::ShowModal(ModalAction::AddMaster));

        // Fields are empty — Enter should NOT confirm, should move to next field
        let action = app.handle_key(key(KeyCode::Enter));
        assert!(matches!(action, Action::None));
        assert_eq!(app.modal.as_ref().unwrap().focused, 1); // moved to next field
    }

    // ─── Data Flow Tests ──────────────────────────────────────────

    #[test]
    fn test_data_loaded_updates_fleet() {
        let mut app = app_in_fleet();
        let new_agents = vec![AgentOverview {
            id: "agent-001".into(),
            hostname: "web-01".into(),
            status: "online".into(),
            cpu_pct: Some(45.0),
            mem_pct: Some(70.0),
            uptime_secs: Some(90000),
            version: "0.4.0".into(),
            cpu_history: vec![],
        }];

        app.process_action(Action::DataLoaded(new_agents));

        assert_eq!(app.fleet.agents.len(), 1);
        assert_eq!(app.fleet.agents[0].cpu_pct, Some(45.0));
        assert_eq!(app.fleet.agents[0].version, "0.4.0");
    }

    #[test]
    fn test_data_loaded_preserves_sparkline_history() {
        let mut app = app_in_fleet();
        // app already has agent-001 with cpu_history: [10, 20, 30]

        let new_agents = vec![AgentOverview {
            id: "agent-001".into(),
            hostname: "web-01".into(),
            status: "online".into(),
            cpu_pct: Some(50.0),
            mem_pct: Some(70.0),
            uptime_secs: Some(90000),
            version: "0.3.0".into(),
            cpu_history: vec![], // API doesn't send history
        }];

        app.process_action(Action::DataLoaded(new_agents));

        // History should be preserved + new point appended
        assert_eq!(app.fleet.agents[0].cpu_history, vec![10, 20, 30, 50]);
    }

    // ─── Fleet Navigation Tests ───────────────────────────────────

    #[test]
    fn test_fleet_jk_navigation() {
        let mut app = app_in_fleet();
        assert_eq!(app.fleet.selected, 0);

        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(app.fleet.selected, 1);

        app.handle_key(key(KeyCode::Char('j')));
        assert_eq!(app.fleet.selected, 1); // can't go past last

        app.handle_key(key(KeyCode::Char('k')));
        assert_eq!(app.fleet.selected, 0);

        app.handle_key(key(KeyCode::Char('k')));
        assert_eq!(app.fleet.selected, 0); // can't go negative
    }

    #[test]
    fn test_fleet_filter() {
        let mut app = app_in_fleet();

        // Start filtering
        app.handle_key(key(KeyCode::Char('/')));
        assert!(app.fleet.filtering);

        // Type "web"
        app.handle_key(key(KeyCode::Char('w')));
        app.handle_key(key(KeyCode::Char('e')));
        app.handle_key(key(KeyCode::Char('b')));
        assert_eq!(app.fleet.filter, "web");

        // Should filter to 1 agent
        assert_eq!(app.fleet.filtered_agents().len(), 1);
        assert_eq!(app.fleet.filtered_agents()[0].hostname, "web-01");

        // Esc exits filter mode
        app.handle_key(key(KeyCode::Esc));
        assert!(!app.fleet.filtering);
        // Filter stays active
        assert_eq!(app.fleet.filtered_agents().len(), 1);
    }

    // ─── Quit Logic ───────────────────────────────────────────────

    #[test]
    fn test_q_quits_from_home() {
        let mut app = app_with_masters();
        let action = app.handle_key(key(KeyCode::Char('q')));
        assert!(matches!(action, Action::Quit));
    }

    #[test]
    fn test_q_does_not_quit_in_exec() {
        let mut app = app_in_fleet();
        app.process_action(Action::NavigateExec("agent-001".into(), "web-01".into()));

        // 'q' in exec should go to input, not quit
        app.handle_key(key(KeyCode::Char('q')));
        assert_eq!(app.exec.input, "q");
        assert!(!app.quit);
    }
}
