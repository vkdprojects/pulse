use crate::action::Action;
use crate::client::AgentOverview;
use crate::ui::{self, BORDER, theme::Theme};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Row, Table},
};

pub struct FleetScreen {
    pub selected: usize,
    pub agents: Vec<AgentOverview>,
    pub context_name: String,
    pub filter: String,
    pub filtering: bool,
    pub tick: u64,
    pub error_msg: Option<String>,
}

impl FleetScreen {
    pub fn new(context_name: String) -> Self {
        Self {
            selected: 0,
            agents: Vec::new(),
            context_name,
            filter: String::new(),
            filtering: false,
            tick: 0,
            error_msg: None,
        }
    }

    pub fn log_error(&mut self, msg: String) {
        self.error_msg = Some(msg);
    }

    pub fn filtered_agents(&self) -> Vec<&AgentOverview> {
        if self.filter.is_empty() {
            self.agents.iter().collect()
        } else {
            let f = self.filter.to_lowercase();
            self.agents.iter().filter(|a| a.hostname.to_lowercase().contains(&f)).collect()
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        self.error_msg = None; // Clear error on interaction
        if self.filtering {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => self.filtering = false,
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
            return Action::None;
        }

        let len = self.filtered_agents().len();
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if len > 0 {
                    self.selected = (self.selected + 1).min(len.saturating_sub(1));
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
            }
            KeyCode::Enter => {
                if let Some(agent) = self.filtered_agents().get(self.selected) {
                    return Action::NavigateDetail(agent.id.clone());
                }
            }
            KeyCode::Char('/') => {
                self.filtering = true;
                self.filter.clear();
            }
            KeyCode::Char('u') => return Action::NavigateUpdate,
            KeyCode::Char('n') => return Action::NavigateProvision(crate::screens::provision::ProvisionTarget::Agent),
            KeyCode::Char('r') => {
                if let Some(agent) = self.filtered_agents().get(self.selected) {
                    return Action::ShowModal(crate::action::ModalAction::RenameAgent(
                        agent.id.clone(),
                        agent.hostname.clone(),
                    ));
                }
            }
            KeyCode::Esc => return Action::NavigateHome,
            KeyCode::Char('q') => return Action::Quit,
            _ => {}
        }
        Action::None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::vertical([Constraint::Length(4), Constraint::Min(5), Constraint::Length(2)]).split(area);

        // Header
        let online = self.agents.iter().filter(|a| a.status == "online").count();
        let total = self.agents.len();
        let offline = total - online;

        let header = Paragraph::new(vec![
            Line::from(vec![
                Span::styled(" ◉ pulse ", theme.title()),
                Span::styled(&self.context_name, theme.title()),
                Span::styled("  │  ", theme.dim()),
                Span::styled(format!("{total} agents"), theme.muted()),
                Span::styled("  ", theme.muted()),
                Span::styled(format!("● {online}"), theme.online()),
                Span::styled("  ", theme.muted()),
                if offline > 0 {
                    Span::styled(format!("○ {offline}"), theme.offline())
                } else {
                    Span::styled("", theme.muted())
                },
            ]),
            Line::from(""),
        ])
        .block(Block::default().borders(Borders::BOTTOM).border_type(BORDER).border_style(theme.border()));
        frame.render_widget(header, chunks[0]);

        // Empty state
        if self.agents.is_empty() {
            let empty = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled("  Nenhum servidor monitorado ainda.", theme.normal())),
                Line::from(""),
                Line::from(Span::styled(
                    "  Cada \"agent\" é um daemon leve (~3MB, ~2MB RAM) que coleta métricas",
                    theme.muted(),
                )),
                Line::from(Span::styled(
                    "  e envia para o master. Instale em cada servidor que deseja monitorar.",
                    theme.muted(),
                )),
                Line::from(""),
                Line::from(Span::styled("  Pressione  n  para instalar um agent via SSH.", theme.muted())),
            ]);
            frame.render_widget(empty, chunks[1]);
        } else {
            // Agent table
            let filtered = self.filtered_agents();
            let rows: Vec<Row> = filtered
                .iter()
                .enumerate()
                .map(|(i, a)| {
                    let sel = i == self.selected;
                    let style = if sel { theme.selected() } else { theme.normal() };
                    let status_span = if a.status == "online" { "●" } else { "○" };
                    let status_color = if a.status == "online" { theme.green } else { theme.red };
                    let cpu_str = a.cpu_pct.map(|v| format!("{:>3.0}%", v)).unwrap_or_else(|| "  —".into());
                    let mem_str = a.mem_pct.map(|v| format!("{:>3.0}%", v)).unwrap_or_else(|| "  —".into());
                    let uptime_str = a.uptime_secs.map(ui::format_uptime).unwrap_or_else(|| "—".into());
                    let hist: Vec<u64> = a.cpu_history.iter().copied().collect();
                    let spark = ui::sparkline_str(&hist, 10);

                    Row::new(vec![
                        Line::from(Span::styled(status_span, Style::default().fg(status_color))),
                        Line::from(Span::styled(format!("{:<16}", a.hostname), style)),
                        Line::from(Span::styled(spark, Style::default().fg(theme.peach))),
                        Line::from(Span::styled(
                            cpu_str,
                            Style::default().fg(theme.gauge_color(a.cpu_pct.unwrap_or(0.0))),
                        )),
                        Line::from(Span::styled(
                            mem_str,
                            Style::default().fg(theme.gauge_color(a.mem_pct.unwrap_or(0.0))),
                        )),
                        Line::from(Span::styled(format!("{:>7}", uptime_str), theme.muted())),
                        Line::from(Span::styled(a.version.clone(), theme.dim())),
                    ])
                    .style(if sel { theme.selected() } else { Style::default() })
                })
                .collect();

            let header_style = theme.dim();
            let table = Table::new(
                rows,
                [
                    Constraint::Length(2),
                    Constraint::Min(16),
                    Constraint::Length(10),
                    Constraint::Length(5),
                    Constraint::Length(5),
                    Constraint::Length(7),
                    Constraint::Length(6),
                ],
            )
            .header(Row::new(vec!["", "HOST", "CPU ─────", "CPU", "MEM", "UPTIME", "VER"]).style(header_style));
            frame.render_widget(table, chunks[1]);
        } // end else (has agents)

        // Footer (show error if any)
        let footer = if let Some(ref err) = self.error_msg {
            Line::from(Span::styled(format!("  ✗ {err}"), theme.offline()))
        } else if self.filtering {
            Line::from(vec![
                Span::styled("  /", theme.key_style()),
                Span::styled(self.filter.clone(), theme.normal()),
                Span::styled("█", theme.muted()),
            ])
        } else {
            ui::footer_line(
                &[
                    ("j/k", "nav"),
                    ("⏎", "detalhe"),
                    ("/", "filtro"),
                    ("n", "add agent"),
                    ("r", "renomear"),
                    ("u", "updates"),
                    ("Esc", "voltar"),
                ],
                theme,
            )
        };
        let footer_widget = Paragraph::new(footer)
            .block(Block::default().borders(Borders::TOP).border_type(BORDER).border_style(theme.border()));
        frame.render_widget(footer_widget, chunks[2]);
    }
}
