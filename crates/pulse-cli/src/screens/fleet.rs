use crate::action::Action;
use crate::client::AgentOverview;
use crate::ui::{self, theme::Theme};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table},
};

pub struct FleetScreen {
    pub selected: usize,
    pub agents: Vec<AgentOverview>,
    pub context_name: String,
    pub filter: String,
    pub filtering: bool,
}

impl FleetScreen {
    pub fn new(context_name: String) -> Self {
        Self { selected: 0, agents: Vec::new(), context_name, filter: String::new(), filtering: false }
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
            KeyCode::Esc => return Action::NavigateHome,
            KeyCode::Char('q') => return Action::Quit,
            _ => {}
        }
        Action::None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(5), Constraint::Length(1)]).split(area);

        // Header
        let online = self.agents.iter().filter(|a| a.status == "online").count();
        let total = self.agents.len();
        let header = Paragraph::new(Line::from(vec![
            Span::styled("⚡ ", theme.header()),
            Span::styled(&self.context_name, theme.header()),
            Span::styled(
                format!("  │  {} agents  │  {} online  │  {} offline", total, online, total - online),
                theme.muted(),
            ),
        ]))
        .block(Block::default().borders(Borders::BOTTOM).border_style(theme.muted()));
        frame.render_widget(header, chunks[0]);

        // Agent table
        let filtered = self.filtered_agents();
        let rows: Vec<Row> = filtered
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let style = if i == self.selected { theme.selected() } else { theme.normal() };
                let status = if a.status == "online" {
                    Span::styled("●", theme.online())
                } else {
                    Span::styled("○", theme.offline())
                };
                let cpu_str = a.cpu_pct.map(|v| format!("{:.0}%", v)).unwrap_or_else(|| "—".into());
                let mem_str = a.mem_pct.map(|v| format!("{:.0}%", v)).unwrap_or_else(|| "—".into());
                let uptime_str = a.uptime_secs.map(ui::format_uptime).unwrap_or_else(|| "—".into());
                let ver = &a.version;

                // Mini sparkline as text (block chars)
                let spark = a.cpu_history_spark();

                Row::new(vec![
                    Cell::from(status),
                    Cell::from(Span::styled(a.hostname.as_str(), style)),
                    Cell::from(Span::styled(spark, theme.muted())),
                    Cell::from(cpu_str),
                    Cell::from(mem_str),
                    Cell::from(uptime_str),
                    Cell::from(Span::styled(ver.as_str(), theme.muted())),
                ])
                .style(style)
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(2),
                Constraint::Min(14),
                Constraint::Length(12),
                Constraint::Length(5),
                Constraint::Length(5),
                Constraint::Length(8),
                Constraint::Length(7),
            ],
        )
        .header(Row::new(vec!["", "HOST", "CPU ─────────", "CPU", "MEM", "UPTIME", "VER"]).style(theme.muted()));
        frame.render_widget(table, chunks[1]);

        // Footer
        let footer = if self.filtering {
            Line::from(vec![
                Span::styled("filter: ", theme.muted()),
                Span::styled(format!("{}█", self.filter), theme.normal()),
            ])
        } else {
            ui::footer_line(
                &[("j/k", "nav"), ("Enter", "detalhe"), ("/", "filtrar"), ("u", "updates"), ("Esc", "voltar")],
                theme,
            )
        };
        frame.render_widget(Paragraph::new(footer), chunks[2]);
    }
}
