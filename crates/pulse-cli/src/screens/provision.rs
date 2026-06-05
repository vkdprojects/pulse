use crate::action::Action;
use crate::ssh::{self, SshEvent};
use crate::ui::theme::Theme;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProvisionTarget {
    Master,
    Agent,
}

pub struct ProvisionScreen {
    pub target: ProvisionTarget,
    pub fields: Vec<Field>,
    pub focused: usize,
    pub log: Vec<String>,
    pub running: bool,
    pub done: bool,
    pub result_token: Option<String>,
    ssh_rx: Option<mpsc::Receiver<SshEvent>>,
    // For agent provisioning
    pub master_url: String,
    pub register_token: String,
}

pub struct Field {
    pub label: &'static str,
    pub value: String,
    pub placeholder: &'static str,
    pub is_password: bool,
}

impl ProvisionScreen {
    pub fn new(target: ProvisionTarget, master_url: String, register_token: String) -> Self {
        let mut fields = vec![
            Field { label: "Host", value: String::new(), placeholder: "192.168.1.100", is_password: false },
            Field { label: "Port", value: "22".into(), placeholder: "22", is_password: false },
            Field { label: "User", value: "root".into(), placeholder: "root", is_password: false },
            Field { label: "Password", value: String::new(), placeholder: "••••••", is_password: true },
        ];
        if target == ProvisionTarget::Agent {
            fields.push(Field {
                label: "Token",
                value: register_token.clone(),
                placeholder: "register token do master",
                is_password: false,
            });
        }
        Self {
            target,
            fields,
            focused: 0,
            log: Vec::new(),
            running: false,
            done: false,
            result_token: None,
            ssh_rx: None,
            master_url,
            register_token,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        if self.running || self.done {
            if key.code == KeyCode::Esc || key.code == KeyCode::Enter {
                return Action::NavigateHome;
            }
            return Action::None;
        }

        match key.code {
            KeyCode::Esc => return Action::NavigateHome,
            KeyCode::Tab => self.focused = (self.focused + 1) % self.fields.len(),
            KeyCode::BackTab => {
                self.focused = self.focused.checked_sub(1).unwrap_or(self.fields.len() - 1);
            }
            KeyCode::Backspace => {
                self.fields[self.focused].value.pop();
            }
            KeyCode::Char(c) => {
                self.fields[self.focused].value.push(c);
            }
            KeyCode::Enter => {
                // Validate and start provisioning
                if !self.fields[0].value.is_empty() && !self.fields[3].value.is_empty() {
                    self.start_provision();
                }
            }
            _ => {}
        }
        Action::None
    }

    fn start_provision(&mut self) {
        let host = self.fields[0].value.clone();
        let port: u16 = self.fields[1].value.parse().unwrap_or(22);
        let user = self.fields[2].value.clone();
        let password = self.fields[3].value.clone();
        let (tx, rx) = mpsc::channel(64);
        self.ssh_rx = Some(rx);
        self.running = true;
        self.log.push("Iniciando provisionamento...".into());

        let master_url = self.master_url.clone();
        // For agent, token is in field[4]; for master, empty
        let register_token = if self.target == ProvisionTarget::Agent {
            self.fields.get(4).map(|f| f.value.clone()).unwrap_or_default()
        } else {
            String::new()
        };
        let target = self.target;

        tokio::spawn(async move {
            match target {
                ProvisionTarget::Master => {
                    ssh::provision_master(&host, port, &user, &password, tx).await;
                }
                ProvisionTarget::Agent => {
                    ssh::provision_agent(&host, port, &user, &password, &master_url, &register_token, tx).await;
                }
            }
        });
    }

    /// Poll SSH events (call from main loop)
    pub fn poll_ssh(&mut self) {
        let Some(ref mut rx) = self.ssh_rx else { return };
        while let Ok(event) = rx.try_recv() {
            match event {
                SshEvent::Log(msg) => self.log.push(msg),
                SshEvent::Done(result) => {
                    self.running = false;
                    self.done = true;
                    match result {
                        Ok(token) => {
                            self.result_token = Some(token);
                            self.log.push("✓ Completo!".into());
                        }
                        Err(e) => self.log.push(format!("✗ Erro: {e}")),
                    }
                }
            }
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(10),
            Constraint::Min(5),
            Constraint::Length(1),
        ])
        .split(area);

        // Header
        let title = match self.target {
            ProvisionTarget::Master => "Instalar Master via SSH",
            ProvisionTarget::Agent => "Instalar Agent via SSH",
        };
        let header = Paragraph::new(Line::from(Span::styled(format!("⚡ {title}"), theme.title())))
            .block(Block::default().borders(Borders::BOTTOM).border_style(theme.muted()));
        frame.render_widget(header, chunks[0]);

        // Form
        let mut form_lines = Vec::new();
        for (i, field) in self.fields.iter().enumerate() {
            let focused = i == self.focused && !self.running;
            let label_style = if focused { theme.title() } else { theme.muted() };
            let val_style = if field.value.is_empty() && !focused { theme.muted() } else { theme.normal() };
            let cursor = if focused { "█" } else { "" };

            let display = if field.is_password && !field.value.is_empty() {
                "•".repeat(field.value.len())
            } else if field.value.is_empty() {
                field.placeholder.to_string()
            } else {
                field.value.clone()
            };

            form_lines.push(Line::from(vec![
                Span::styled(format!("  {:<10}", field.label), label_style),
                Span::styled(display, val_style),
                Span::styled(cursor.to_string(), theme.normal()),
            ]));
        }
        let form = Paragraph::new(form_lines)
            .block(Block::default().borders(Borders::ALL).border_style(theme.muted()).title(" Credenciais "));
        frame.render_widget(form, chunks[1]);

        // Log output
        let log_lines: Vec<Line> = self
            .log
            .iter()
            .map(|l| {
                let style = if l.starts_with('✓') {
                    theme.online()
                } else if l.starts_with('✗') {
                    theme.offline()
                } else {
                    theme.muted()
                };
                Line::from(Span::styled(format!("  {l}"), style))
            })
            .collect();
        let log = Paragraph::new(log_lines)
            .wrap(Wrap { trim: false })
            .block(Block::default().borders(Borders::ALL).border_style(theme.muted()).title(" Output "));
        frame.render_widget(log, chunks[2]);

        // Footer
        let footer_text = if self.running {
            "Provisionando..."
        } else if self.done {
            "[Enter/Esc] voltar"
        } else {
            "[Tab] próximo  [Enter] instalar  [Esc] cancelar"
        };
        let footer = Paragraph::new(Span::styled(footer_text, theme.desc_style()));
        frame.render_widget(footer, chunks[3]);
    }
}
