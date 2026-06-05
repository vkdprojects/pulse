use crate::action::Action;
use crate::installer::{self, InstallEvent};
use crate::ui::{BORDER, theme::Theme};
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
    pub result: Option<installer::InstallResult>,
    event_rx: Option<mpsc::Receiver<InstallEvent>>,
    github_token: String,
    github_repo: String,
    master_url: String,
    register_token: String,
}

pub struct Field {
    pub label: &'static str,
    pub value: String,
    pub placeholder: &'static str,
    pub is_password: bool,
}

impl ProvisionScreen {
    pub fn new(
        target: ProvisionTarget,
        github_token: String,
        github_repo: String,
        master_url: String,
        register_token: String,
    ) -> Self {
        let fields = vec![
            Field { label: "Host", value: String::new(), placeholder: "192.168.1.100", is_password: false },
            Field { label: "Port", value: "22".into(), placeholder: "22", is_password: false },
            Field { label: "User", value: "root".into(), placeholder: "root", is_password: false },
            Field { label: "Auth", value: "password".into(), placeholder: "password | key", is_password: false },
            Field { label: "Secret", value: String::new(), placeholder: "password or key path", is_password: true },
        ];
        Self {
            target,
            fields,
            focused: 0,
            log: Vec::new(),
            running: false,
            done: false,
            result: None,
            event_rx: None,
            github_token,
            github_repo,
            master_url,
            register_token,
        }
    }

    /// Pre-fill from an SSH config host entry
    pub fn fill_from_ssh_host(&mut self, host: &crate::ssh_config::SshHost) {
        self.fields[0].value = host.hostname.clone();
        self.fields[1].value = host.port.to_string();
        if !host.user.is_empty() {
            self.fields[2].value = host.user.clone();
        }
        if let Some(ref key) = host.identity_file {
            self.fields[3].value = "key".into();
            self.fields[4].value = key.clone();
            self.fields[4].is_password = false;
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        if self.done {
            if key.code == KeyCode::Enter || key.code == KeyCode::Esc {
                return Action::NavigateHome;
            }
            return Action::None;
        }
        if self.running {
            return Action::None; // can't interact while installing
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
                if !self.fields[0].value.is_empty() && !self.fields[4].value.is_empty() {
                    self.start_install();
                }
            }
            _ => {}
        }
        Action::None
    }

    fn start_install(&mut self) {
        let host = self.fields[0].value.clone();
        let port: u16 = self.fields[1].value.parse().unwrap_or(22);
        let user = self.fields[2].value.clone();
        let auth_mode = self.fields[3].value.clone();
        let secret = self.fields[4].value.clone();

        let auth =
            if auth_mode == "key" { installer::SshAuth::Key(secret) } else { installer::SshAuth::Password(secret) };

        let (tx, rx) = mpsc::channel(64);
        self.event_rx = Some(rx);
        self.running = true;
        self.log.clear();

        let opts = installer::InstallOpts {
            host,
            port,
            user,
            auth,
            github_token: self.github_token.clone(),
            github_repo: self.github_repo.clone(),
            master_url: if self.target == ProvisionTarget::Agent { Some(self.master_url.clone()) } else { None },
            register_token: if self.target == ProvisionTarget::Agent {
                Some(self.register_token.clone())
            } else {
                None
            },
        };

        let target = self.target;
        tokio::spawn(async move {
            match target {
                ProvisionTarget::Master => installer::install_master(opts, tx).await,
                ProvisionTarget::Agent => installer::install_agent(opts, tx).await,
            }
        });
    }

    /// Poll install events (call from main loop tick)
    pub fn poll_events(&mut self) {
        let Some(ref mut rx) = self.event_rx else { return };
        while let Ok(event) = rx.try_recv() {
            match event {
                InstallEvent::Progress(msg) => self.log.push(msg),
                InstallEvent::Done(result) => {
                    self.running = false;
                    self.done = true;
                    match result {
                        Ok(r) => {
                            self.log.push("".into());
                            self.log.push("✓ Instalação completa!".into());
                            self.result = Some(r);
                        }
                        Err(e) => {
                            self.log.push(format!("✗ Erro: {e}"));
                        }
                    }
                }
            }
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let explanation = match self.target {
            ProvisionTarget::Master => vec![
                "  Baixa o binário do GitHub, faz upload via SFTP e configura systemd.",
                "  Requisitos: Linux (x86_64/arm64), SSH com sudo, porta 8080 e 50051 livres.",
            ],
            ProvisionTarget::Agent => vec![
                "  Instala o agent que coleta métricas e envia para o master.",
                "  Requisitos: Linux (x86_64/arm64), SSH com sudo.",
            ],
        };

        let header_height = 4 + explanation.len() as u16;
        let chunks = Layout::vertical([
            Constraint::Length(header_height),
            Constraint::Length(self.fields.len() as u16 + 2),
            Constraint::Min(5),
            Constraint::Length(2),
        ])
        .split(area);

        // Header with explanation
        let title = match self.target {
            ProvisionTarget::Master => "Instalar Master via SSH",
            ProvisionTarget::Agent => "Instalar Agent via SSH",
        };
        let mut header_lines = vec![
            Line::from(vec![Span::styled("  ⚡ ", theme.title()), Span::styled(title, theme.title())]),
            Line::from(""),
        ];
        for line in &explanation {
            header_lines.push(Line::from(Span::styled(*line, theme.muted())));
        }
        let header = Paragraph::new(header_lines)
            .block(Block::default().borders(Borders::BOTTOM).border_type(BORDER).border_style(theme.border()));
        frame.render_widget(header, chunks[0]);

        // Form
        let mut form_lines = Vec::new();
        for (i, field) in self.fields.iter().enumerate() {
            let focused = i == self.focused && !self.running;
            let label_style = if focused { theme.title() } else { theme.muted() };
            let display = if field.is_password && !field.value.is_empty() {
                "•".repeat(field.value.len())
            } else if field.value.is_empty() {
                field.placeholder.to_string()
            } else {
                field.value.clone()
            };
            let val_style = if field.value.is_empty() && !focused { theme.dim() } else { theme.normal() };
            let cursor = if focused { "█" } else { "" };
            form_lines.push(Line::from(vec![
                Span::styled(format!("  {:<10}", field.label), label_style),
                Span::styled(display, val_style),
                Span::styled(cursor.to_string(), theme.normal()),
            ]));
        }
        let form = Paragraph::new(form_lines).block(
            Block::default().borders(Borders::ALL).border_type(BORDER).border_style(theme.border()).title(" SSH "),
        );
        frame.render_widget(form, chunks[1]);

        // Log output
        let max_lines = chunks[2].height as usize;
        let visible: Vec<Line> = self
            .log
            .iter()
            .rev()
            .take(max_lines)
            .rev()
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
        let log = Paragraph::new(visible).wrap(Wrap { trim: false }).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BORDER)
                .border_style(theme.border())
                .title(" Progresso "),
        );
        frame.render_widget(log, chunks[2]);

        // Footer
        let footer_text = if self.running {
            "  Instalando..."
        } else if self.done {
            "  [Enter] concluir"
        } else {
            "  [Tab] próximo │ [Enter] instalar │ [Esc] cancelar"
        };
        let footer = Paragraph::new(Span::styled(footer_text, theme.muted()))
            .block(Block::default().borders(Borders::TOP).border_type(BORDER).border_style(theme.border()));
        frame.render_widget(footer, chunks[3]);
    }
}
