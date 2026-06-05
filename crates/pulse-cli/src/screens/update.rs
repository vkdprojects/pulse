use crate::action::Action;
use crate::client::{AgentOverview, Client};
use crate::ui::{self, theme::Theme};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

pub struct UpdateScreen {
    pub agents: Vec<AgentOverview>,
    pub target_version: String,
    pub rollout_pct: u8,
}

impl UpdateScreen {
    pub fn new() -> Self {
        Self { agents: Vec::new(), target_version: String::new(), rollout_pct: 100 }
    }

    pub async fn refresh(&mut self, client: &Client) {
        if let Ok(agents) = client.overview().await {
            self.agents = agents;
        }
        if let Ok(v) = client.get_rollout().await
            && let Some(cfg) = v.get("config")
        {
            self.target_version = cfg.get("target_version").and_then(|v| v.as_str()).unwrap_or("").to_string();
            self.rollout_pct = cfg.get("rollout_pct").and_then(|v| v.as_u64()).unwrap_or(100) as u8;
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => return Action::NavigateFleet(String::new()),
            KeyCode::Char('q') => return Action::Quit,
            _ => {}
        }
        Action::None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(5), Constraint::Length(1)]).split(area);

        // Header
        let target = if self.target_version.is_empty() { "none" } else { &self.target_version };
        let updated = self.agents.iter().filter(|a| a.version == self.target_version).count();
        let total = self.agents.len();
        let header = Paragraph::new(Line::from(vec![
            Span::styled("⚡ Fleet Update", theme.title()),
            Span::styled(
                format!("  │  Target: {target}  │  Rollout: {}%  │  {updated}/{total} atualizado", self.rollout_pct),
                theme.muted(),
            ),
        ]))
        .block(Block::default().borders(Borders::BOTTOM).border_style(theme.muted()));
        frame.render_widget(header, chunks[0]);

        // Version table
        let rows: Vec<Row> = self
            .agents
            .iter()
            .map(|a| {
                let (icon, style) = if a.version == self.target_version || self.target_version.is_empty() {
                    ("✅", theme.online())
                } else if a.status == "offline" {
                    ("❌", theme.offline())
                } else {
                    ("⏳", theme.normal())
                };
                Row::new(vec![
                    Cell::from(Span::styled(icon, style)),
                    Cell::from(a.hostname.as_str()),
                    Cell::from(Span::styled(a.version.as_str(), style)),
                    Cell::from(target),
                    Cell::from(a.status.as_str()),
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(3),
                Constraint::Min(14),
                Constraint::Length(8),
                Constraint::Length(8),
                Constraint::Length(8),
            ],
        )
        .header(Row::new(vec!["", "HOST", "CURRENT", "TARGET", "STATUS"]).style(theme.muted()))
        .block(Block::default().borders(Borders::NONE));
        frame.render_widget(table, chunks[1]);

        let footer = ui::footer_line(&[("Esc", "voltar"), ("q", "sair")], theme);
        frame.render_widget(Paragraph::new(footer), chunks[2]);
    }
}

impl Client {
    pub async fn get_rollout(&self) -> anyhow::Result<serde_json::Value> {
        let mut req = self.http.get(format!("{}/api/updates/rollout", self.base_url));
        if let Some(ref t) = self.token {
            req = req.bearer_auth(t);
        }
        Ok(req.send().await?.json().await?)
    }
}
