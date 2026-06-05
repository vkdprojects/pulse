use crate::action::Action;
use crate::ui::{self, BORDER, theme::Theme};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

pub struct SetupScreen {
    pub token_input: String,
    pub repo_input: String,
    pub focused: usize, // 0 = token, 1 = repo
    pub done: bool,
}

impl SetupScreen {
    pub fn new() -> Self {
        Self { token_input: String::new(), repo_input: "gcnsolucoes/vixpi-pulse".into(), focused: 0, done: false }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Tab => self.focused = (self.focused + 1) % 2,
            KeyCode::BackTab => self.focused = if self.focused == 0 { 1 } else { 0 },
            KeyCode::Backspace => {
                if self.focused == 0 {
                    self.token_input.pop();
                } else {
                    self.repo_input.pop();
                }
            }
            KeyCode::Char(c) => {
                if self.focused == 0 {
                    self.token_input.push(c);
                } else {
                    self.repo_input.push(c);
                }
            }
            KeyCode::Enter => {
                if !self.token_input.is_empty() {
                    self.done = true;
                    return Action::CloseModal; // signal to main.rs to save config
                }
            }
            KeyCode::Esc => {
                // Skip setup — go to home without token (manual mode)
                self.done = true;
                self.token_input.clear();
                return Action::CloseModal;
            }
            _ => {}
        }
        Action::None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks =
            Layout::vertical([Constraint::Length(6), Constraint::Length(8), Constraint::Min(3), Constraint::Length(2)])
                .split(area);

        // Header
        let header = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled("  ⚡ Bem-vindo ao Pulse!", theme.title())),
            Line::from(""),
            Line::from(Span::styled("  Vamos configurar o acesso ao repositório.", theme.muted())),
        ])
        .block(Block::default().borders(Borders::BOTTOM).border_type(BORDER).border_style(theme.border()));
        frame.render_widget(header, chunks[0]);

        // Form
        let token_style = if self.focused == 0 { theme.title() } else { theme.muted() };
        let repo_style = if self.focused == 1 { theme.title() } else { theme.muted() };
        let cursor0 = if self.focused == 0 { "█" } else { "" };
        let cursor1 = if self.focused == 1 { "█" } else { "" };

        let token_display = if self.token_input.is_empty() {
            "ghp_xxxxxxxxxxxxxxxx".to_string()
        } else {
            format!("{}...", &self.token_input[..self.token_input.len().min(20)])
        };

        let form = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled("  GitHub Token (scope: repo):", token_style)),
            Line::from(vec![
                Span::styled("  ", theme.normal()),
                Span::styled(&token_display, if self.token_input.is_empty() { theme.dim() } else { theme.normal() }),
                Span::styled(cursor0, theme.normal()),
            ]),
            Line::from(""),
            Line::from(Span::styled("  Repositório:", repo_style)),
            Line::from(vec![
                Span::styled("  ", theme.normal()),
                Span::styled(&self.repo_input, theme.normal()),
                Span::styled(cursor1, theme.normal()),
            ]),
        ]);
        frame.render_widget(form, chunks[1]);

        // Help text
        let help = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled("  O token permite baixar binários do repo privado.", theme.muted())),
            Line::from(Span::styled("  Crie em: github.com/settings/tokens", theme.muted())),
            Line::from(Span::styled("  Scope necessário: repo (full control)", theme.muted())),
        ])
        .wrap(Wrap { trim: false });
        frame.render_widget(help, chunks[2]);

        // Footer
        let footer = ui::footer_line(&[("Tab", "próximo"), ("Enter", "salvar"), ("Esc", "pular")], theme);
        let fw = Paragraph::new(footer)
            .block(Block::default().borders(Borders::TOP).border_type(BORDER).border_style(theme.border()));
        frame.render_widget(fw, chunks[3]);
    }
}
