use crate::action::Action;
use crate::config::Config;
use crate::ui::{self, BORDER, theme::Theme};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

pub struct HomeScreen {
    pub selected: usize,
}

impl HomeScreen {
    pub fn new() -> Self {
        Self { selected: 0 }
    }

    pub fn handle_key(&mut self, key: KeyEvent, config: &Config) -> Action {
        let total = config.masters.len() + 1;
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.selected = (self.selected + 1).min(total.saturating_sub(1));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
            }
            KeyCode::Enter => {
                if self.selected < config.masters.len() {
                    return Action::NavigateFleet(config.masters[self.selected].name.clone());
                }
                return Action::ShowModal(crate::action::ModalAction::AddMaster);
            }
            KeyCode::Char('n') => return Action::ShowModal(crate::action::ModalAction::AddMaster),
            KeyCode::Char('s') => return Action::NavigateProvision(crate::screens::provision::ProvisionTarget::Master),
            KeyCode::Char('u') => {
                // Update master - extract host from URL and go straight to SSH form
                if self.selected < config.masters.len() {
                    let master = &config.masters[self.selected];
                    // Extract host from URL: http://192.168.68.66:8080 → 192.168.68.66
                    let host = master
                        .url
                        .replace("http://", "")
                        .replace("https://", "")
                        .split(':')
                        .next()
                        .unwrap_or("")
                        .to_string();
                    return Action::UpdateMaster(host);
                }
            }
            KeyCode::Char('r') => {
                if self.selected < config.masters.len() {
                    let name = config.masters[self.selected].name.clone();
                    return Action::ShowModal(crate::action::ModalAction::RenameMaster(name));
                }
            }
            KeyCode::Char('d') => {
                if self.selected < config.masters.len() {
                    let name = config.masters[self.selected].name.clone();
                    return Action::ShowModal(crate::action::ModalAction::ConfirmDelete(name));
                }
            }
            KeyCode::Char('q') => return Action::Quit,
            _ => {}
        }
        Action::None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, config: &Config, theme: &Theme) {
        let chunks = Layout::vertical([Constraint::Length(4), Constraint::Min(5), Constraint::Length(2)]).split(area);

        // Header with logo
        let header_lines = vec![
            Line::from(vec![
                Span::styled(" ◉ pulse ", theme.title()),
                Span::styled("Pulse", theme.title()),
                Span::styled("  │  ", theme.dim()),
                Span::styled(format!("{} masters", config.masters.len()), theme.muted()),
            ]),
            Line::from(""),
        ];
        let header = Paragraph::new(header_lines)
            .block(Block::default().borders(Borders::BOTTOM).border_type(BORDER).border_style(theme.border()));
        frame.render_widget(header, chunks[0]);

        // Master list
        let mut lines = Vec::new();
        lines.push(Line::from(""));

        if config.masters.is_empty() {
            // Empty state with explanation
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "  Um \"master\" é o servidor central que recebe métricas de todos os seus servidores.",
                theme.muted(),
            )));
            lines.push(Line::from(Span::styled(
                "  Você precisa de pelo menos um master rodando para começar.",
                theme.muted(),
            )));
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled("  Para começar:", theme.normal())));
            lines.push(Line::from(Span::styled(
                "  • Pressione  s  para instalar um master em um servidor via SSH",
                theme.muted(),
            )));
            lines.push(Line::from(Span::styled(
                "  • Ou pressione  n  para conectar a um master já existente (URL manual)",
                theme.muted(),
            )));
            lines.push(Line::from(""));
        } else {
            for (i, master) in config.masters.iter().enumerate() {
                let selected = i == self.selected;
                let row_style = if selected { theme.selected() } else { theme.normal() };
                let indicator = if selected { "›" } else { " " };

                lines.push(Line::from(vec![
                    Span::styled(format!("  {indicator} "), theme.key_style()),
                    Span::styled("● ", theme.online()),
                    Span::styled(format!("{:<18}", master.name), row_style),
                    Span::styled(&master.url, if selected { row_style } else { theme.muted() }),
                ]));
            }
        }

        // "Add new" option
        let add_idx = config.masters.len();
        let add_selected = self.selected == add_idx;
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(if add_selected { "  › " } else { "    " }, theme.key_style()),
            Span::styled("+ ", theme.teal_style()),
            Span::styled("Adicionar master", if add_selected { theme.selected() } else { theme.muted() }),
        ]));

        let list = Paragraph::new(lines);
        frame.render_widget(list, chunks[1]);

        // Footer
        let footer = ui::footer_line(
            &[
                ("j/k", "navegar"),
                ("⏎", "conectar"),
                ("n", "novo manual"),
                ("s", "instalar SSH"),
                ("u", "update"),
                ("r", "renomear"),
                ("d", "remover"),
                ("q", "sair"),
            ],
            theme,
        );
        let footer_widget = Paragraph::new(footer)
            .block(Block::default().borders(Borders::TOP).border_type(BORDER).border_style(theme.border()));
        frame.render_widget(footer_widget, chunks[2]);
    }
}

// Extension for the teal color (used for + icon)
impl Theme {
    pub fn teal_style(&self) -> Style {
        ratatui::style::Style::default().fg(self.teal)
    }
}

use ratatui::style::Style;
