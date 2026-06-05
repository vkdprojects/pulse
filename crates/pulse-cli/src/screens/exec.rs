use crate::action::Action;
use crate::client::Client;
use crate::ui::{self, theme::Theme};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub struct ExecScreen {
    pub agent_id: String,
    pub hostname: String,
    pub input: String,
    pub output: Vec<String>,
    pub running: bool,
    pub history: Vec<String>,
    pub history_pos: Option<usize>,
}

impl ExecScreen {
    pub fn new(agent_id: String, hostname: String) -> Self {
        Self {
            agent_id,
            hostname,
            input: String::new(),
            output: Vec::new(),
            running: false,
            history: Vec::new(),
            history_pos: None,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => return Action::NavigateDetail(self.agent_id.clone()),
            KeyCode::Enter => {
                if !self.input.is_empty() && !self.running {
                    return Action::None; // Caller will call run_command
                }
            }
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Up => {
                if !self.history.is_empty() {
                    let pos = self.history_pos.map(|p| p.saturating_sub(1)).unwrap_or(self.history.len() - 1);
                    self.history_pos = Some(pos);
                    self.input = self.history[pos].clone();
                }
            }
            KeyCode::Down => {
                if let Some(pos) = self.history_pos {
                    if pos + 1 < self.history.len() {
                        self.history_pos = Some(pos + 1);
                        self.input = self.history[pos + 1].clone();
                    } else {
                        self.history_pos = None;
                        self.input.clear();
                    }
                }
            }
            KeyCode::Char(c) => {
                self.input.push(c);
                self.history_pos = None;
            }
            _ => {}
        }
        Action::None
    }

    pub async fn run_command(&mut self, client: &Client) {
        if self.input.is_empty() {
            return;
        }
        self.running = true;
        let cmd = self.input.clone();
        self.history.push(cmd.clone());
        self.history_pos = None;
        self.output.push(format!("$ {cmd}"));

        match client.exec_command(&self.agent_id, &cmd).await {
            Ok(v) => {
                if let Some(stdout) = v.get("stdout").and_then(|s| s.as_str())
                    && !stdout.is_empty()
                {
                    for line in stdout.lines() {
                        self.output.push(line.to_string());
                    }
                }
                if let Some(stderr) = v.get("stderr").and_then(|s| s.as_str())
                    && !stderr.is_empty()
                {
                    for line in stderr.lines() {
                        self.output.push(format!("[err] {line}"));
                    }
                }
                let code = v.get("exit_code").and_then(|c| c.as_i64()).unwrap_or(-1);
                if code != 0 {
                    self.output.push(format!("[exit: {code}]"));
                }
            }
            Err(e) => self.output.push(format!("[error: {e}]")),
        }
        if self.output.len() > 1000 {
            self.output.drain(..self.output.len() - 800);
        }
        self.input.clear();
        self.running = false;
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks =
            Layout::vertical([Constraint::Length(5), Constraint::Min(5), Constraint::Length(3), Constraint::Length(1)])
                .split(area);

        // Header with explanation
        let header = Paragraph::new(vec![
            Line::from(vec![
                Span::styled(" ◉ pulse ", theme.title()),
                Span::styled(format!("exec @ {}", self.hostname), theme.title()),
            ]),
            Line::from(Span::styled("  Executa comandos no servidor master. Requer admin token.", theme.dim())),
            Line::from(""),
        ])
        .block(Block::default().borders(Borders::BOTTOM).border_type(crate::ui::BORDER).border_style(theme.border()));
        frame.render_widget(header, chunks[0]);

        // Output
        let max_lines = chunks[1].height as usize;
        let visible: Vec<Line> = self
            .output
            .iter()
            .rev()
            .take(max_lines)
            .rev()
            .map(|l| {
                if l.starts_with('$') {
                    Line::from(Span::styled(l.as_str(), theme.title()))
                } else if l.starts_with("[err]") || l.starts_with("[error") {
                    Line::from(Span::styled(l.as_str(), theme.offline()))
                } else {
                    Line::from(Span::styled(l.as_str(), theme.normal()))
                }
            })
            .collect();
        let output = Paragraph::new(visible)
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::ALL).border_style(theme.muted()).title(" Output "));
        frame.render_widget(output, chunks[1]);

        // Input
        let prompt = if self.running { "running..." } else { "$ " };
        let input = Paragraph::new(Line::from(vec![
            Span::styled(prompt, theme.muted()),
            Span::styled(&self.input, theme.normal()),
            Span::styled("█", theme.normal()),
        ]))
        .block(Block::default().borders(Borders::ALL).border_style(theme.blue).title(" Command "));
        frame.render_widget(input, chunks[2]);

        // Footer
        let footer = ui::footer_line(&[("Enter", "executar"), ("↑/↓", "histórico"), ("Esc", "voltar")], theme);
        frame.render_widget(Paragraph::new(footer), chunks[3]);
    }
}

impl Client {
    pub async fn exec_command(&self, agent_id: &str, command: &str) -> anyhow::Result<serde_json::Value> {
        let mut req = self.http.post(format!("{}/api/agents/{agent_id}/exec", self.base_url));
        if let Some(ref t) = self.token {
            req = req.bearer_auth(t);
        }
        let resp = req.json(&serde_json::json!({"command": command})).send().await?;
        if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
            anyhow::bail!("Admin token requerido. Configure o token no master ou exporte PULSE_ADMIN_TOKEN");
        }
        Ok(resp.json().await?)
    }
}
