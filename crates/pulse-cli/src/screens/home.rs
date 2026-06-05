use crate::action::Action;
use crate::config::Config;
use crate::ui::{self, theme::Theme};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Modifier,
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
        let total = config.masters.len() + 1; // +1 for "add new"
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.selected = (self.selected + 1).min(total.saturating_sub(1));
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
            }
            KeyCode::Enter => {
                if self.selected < config.masters.len() {
                    let name = config.masters[self.selected].name.clone();
                    return Action::NavigateFleet(name);
                }
                // "Add new" selected
                return Action::ShowModal(crate::action::ModalAction::AddMaster);
            }
            KeyCode::Char('n') => return Action::ShowModal(crate::action::ModalAction::AddMaster),
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
        let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(5), Constraint::Length(1)]).split(area);

        // Header
        let header = Paragraph::new(Line::from(vec![
            Span::styled("⚡ ", theme.header()),
            Span::styled("Pulse", theme.header()),
            Span::styled(format!("  │  {} masters configurados", config.masters.len()), theme.muted()),
        ]))
        .block(Block::default().borders(Borders::BOTTOM).border_style(theme.muted()));
        frame.render_widget(header, chunks[0]);

        // Master list
        let mut lines = Vec::new();
        for (i, master) in config.masters.iter().enumerate() {
            let selected = i == self.selected;
            let style = if selected { theme.selected() } else { theme.normal() };
            let icon = if selected { "›" } else { " " };
            let status_icon = Span::styled("●", theme.online());
            let name = Span::styled(format!(" {:<16}", master.name), style);
            let url = Span::styled(format!("{:<32}", master.url), if selected { style } else { theme.muted() });
            lines.push(Line::from(vec![Span::styled(icon, theme.shortcut()), Span::raw(" "), status_icon, name, url]));
        }

        // "Add new" option
        let add_selected = self.selected == config.masters.len();
        let add_style = if add_selected { theme.selected() } else { theme.muted() };
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled(if add_selected { "›" } else { " " }, theme.shortcut()),
            Span::styled("  + Adicionar master", add_style.add_modifier(Modifier::ITALIC)),
        ]));

        let list = Paragraph::new(lines).block(Block::default().borders(Borders::NONE));
        frame.render_widget(list, chunks[1]);

        // Footer
        let footer = ui::footer_line(
            &[("j/k", "navegar"), ("Enter", "conectar"), ("n", "novo"), ("d", "remover"), ("q", "sair")],
            theme,
        );
        frame.render_widget(Paragraph::new(footer), chunks[2]);
    }
}
