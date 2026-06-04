use crate::client::{AgentMetric, AgentOverview, Client, DiskInfo, NetInterface, ProcessInfo};
use crossterm::event::{KeyCode, KeyEvent};

#[derive(Clone, Copy, PartialEq)]
pub enum Screen {
    Fleet,
    Detail,
    Update,
    Exec,
}

#[derive(Clone, Copy, PartialEq)]
pub enum DetailTab {
    System,
    Processes,
}

pub struct App {
    pub client: Client,
    pub screen: Screen,
    pub quit: bool,

    // Fleet state
    pub agents: Vec<AgentOverview>,
    pub selected: usize,
    pub filter: String,
    pub filtering: bool,

    // Detail state
    pub detail_tab: DetailTab,
    pub metrics: Vec<AgentMetric>,
    pub processes: Vec<ProcessInfo>,
    pub disks: Vec<DiskInfo>,
    pub network: Vec<NetInterface>,

    // Update state
    pub target_version: String,
    pub rollout_pct: u8,

    // Exec state
    pub exec_input: String,
    pub exec_output: String,
    pub exec_running: bool,

    pub loading: bool,
    pub error: Option<String>,
}

impl App {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            screen: Screen::Fleet,
            quit: false,
            agents: Vec::new(),
            selected: 0,
            filter: String::new(),
            filtering: false,
            detail_tab: DetailTab::System,
            metrics: Vec::new(),
            processes: Vec::new(),
            disks: Vec::new(),
            network: Vec::new(),
            target_version: String::new(),
            rollout_pct: 100,
            exec_input: String::new(),
            exec_output: String::new(),
            exec_running: false,
            loading: true,
            error: None,
        }
    }

    pub fn filtered_agents(&self) -> Vec<&AgentOverview> {
        if self.filter.is_empty() {
            self.agents.iter().collect()
        } else {
            let f = self.filter.to_lowercase();
            self.agents.iter().filter(|a| a.hostname.to_lowercase().contains(&f) || a.id.contains(&f)).collect()
        }
    }

    pub fn selected_agent(&self) -> Option<&AgentOverview> {
        self.filtered_agents().get(self.selected).copied()
    }

    pub async fn refresh_fleet(&mut self) {
        match self.client.overview().await {
            Ok(agents) => {
                self.agents = agents;
                self.error = None;
            }
            Err(e) => self.error = Some(e.to_string()),
        }
        self.loading = false;
    }

    pub async fn refresh_detail(&mut self) {
        let Some(agent) = self.selected_agent().map(|a| a.id.clone()) else { return };
        let (metrics, procs, disks, net) = tokio::join!(
            self.client.agent_metrics(&agent),
            self.client.agent_processes(&agent),
            self.client.agent_disks(&agent),
            self.client.agent_network(&agent),
        );
        self.metrics = metrics.unwrap_or_default();
        self.processes = procs.unwrap_or_default();
        self.disks = disks.unwrap_or_default();
        self.network = net.unwrap_or_default();
    }

    pub async fn refresh_rollout(&mut self) {
        if let Ok(v) = self.client.get_rollout().await
            && let Some(cfg) = v.get("config")
        {
            self.target_version = cfg.get("target_version").and_then(|v| v.as_str()).unwrap_or("").to_string();
            self.rollout_pct = cfg.get("rollout_pct").and_then(|v| v.as_u64()).unwrap_or(100) as u8;
        }
    }

    /// Returns true if async action needed (detail refresh, exec, etc)
    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        if self.filtering {
            self.handle_filter_key(key);
            return Action::None;
        }
        match self.screen {
            Screen::Fleet => self.handle_fleet_key(key),
            Screen::Detail => self.handle_detail_key(key),
            Screen::Update => self.handle_update_key(key),
            Screen::Exec => self.handle_exec_key(key),
        }
    }

    fn handle_fleet_key(&mut self, key: KeyEvent) -> Action {
        let len = self.filtered_agents().len();
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if len > 0 {
                    self.selected = (self.selected + 1).min(len - 1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
            }
            KeyCode::Enter => {
                if self.selected_agent().is_some() {
                    self.screen = Screen::Detail;
                    self.detail_tab = DetailTab::System;
                    return Action::RefreshDetail;
                }
            }
            KeyCode::Char('/') => {
                self.filtering = true;
                self.filter.clear();
            }
            KeyCode::Char('u') => {
                self.screen = Screen::Update;
                return Action::RefreshRollout;
            }
            _ => {}
        }
        Action::None
    }

    fn handle_detail_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => {
                self.screen = Screen::Fleet;
            }
            KeyCode::Tab | KeyCode::Char('2') => {
                self.detail_tab = match self.detail_tab {
                    DetailTab::System => DetailTab::Processes,
                    DetailTab::Processes => DetailTab::System,
                };
            }
            KeyCode::Char('1') => self.detail_tab = DetailTab::System,
            KeyCode::Char('e') => {
                self.screen = Screen::Exec;
                self.exec_input.clear();
                self.exec_output.clear();
            }
            _ => {}
        }
        Action::None
    }

    fn handle_update_key(&mut self, key: KeyEvent) -> Action {
        if key.code == KeyCode::Esc {
            self.screen = Screen::Fleet;
        }
        Action::None
    }

    fn handle_exec_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => self.screen = Screen::Detail,
            KeyCode::Enter => {
                if !self.exec_input.is_empty() && !self.exec_running {
                    return Action::RunExec;
                }
            }
            KeyCode::Backspace => {
                self.exec_input.pop();
            }
            KeyCode::Char(c) => {
                self.exec_input.push(c);
            }
            _ => {}
        }
        Action::None
    }

    fn handle_filter_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                self.filtering = false;
            }
            KeyCode::Backspace => {
                self.filter.pop();
                self.selected = 0;
            }
            KeyCode::Char(c) => {
                self.filter.push(c);
                self.selected = 0;
            }
            _ => {}
        }
    }

    pub async fn run_exec(&mut self) {
        let Some(agent) = self.selected_agent().map(|a| a.id.clone()) else { return };
        self.exec_running = true;
        self.exec_output = "Running...".into();
        match self.client.exec_command(&agent, &self.exec_input).await {
            Ok(v) => {
                let stdout = v.get("stdout").and_then(|s| s.as_str()).unwrap_or("");
                let stderr = v.get("stderr").and_then(|s| s.as_str()).unwrap_or("");
                let code = v.get("exit_code").and_then(|c| c.as_i64()).unwrap_or(-1);
                let timed_out = v.get("timed_out").and_then(|t| t.as_bool()).unwrap_or(false);
                let mut output = String::new();
                if !stdout.is_empty() {
                    output.push_str(stdout);
                }
                if !stderr.is_empty() {
                    if !output.is_empty() {
                        output.push('\n');
                    }
                    output.push_str(&format!("[stderr] {stderr}"));
                }
                if timed_out {
                    output.push_str("\n[timed out]");
                }
                output.push_str(&format!("\n\n[exit code: {code}]"));
                self.exec_output = output;
            }
            Err(e) => self.exec_output = format!("Error: {e}"),
        }
        self.exec_running = false;
    }
}

pub enum Action {
    None,
    RefreshDetail,
    RefreshRollout,
    RunExec,
}
