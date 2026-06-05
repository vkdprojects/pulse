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
    pub focused: usize,
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
                    return Action::CloseModal;
                }
            }
            KeyCode::Esc => {
                self.done = true;
                self.token_input.clear();
                return Action::CloseModal;
            }
            _ => {}
        }
        Action::None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::vertical([
            Constraint::Length(10),
            Constraint::Length(11),
            Constraint::Min(3),
            Constraint::Length(2),
        ])
        .split(area);

        // Welcome header with logo + explanation
        let mut header_lines = ui::logo_lines(theme);
        header_lines.push(Line::from(""));
        header_lines.push(Line::from(Span::styled(
            "  Pulse monitora seus servidores: CPU, memória, disco, rede — tudo em tempo real.",
            theme.normal(),
        )));
        header_lines.push(Line::from(Span::styled(
            "  Para começar, precisamos acessar o repositório para baixar os binários.",
            theme.muted(),
        )));
        let header = Paragraph::new(header_lines)
            .block(Block::default().borders(Borders::BOTTOM).border_type(BORDER).border_style(theme.border()));
        frame.render_widget(header, chunks[0]);

        // Form with explanations
        let token_style = if self.focused == 0 { theme.title() } else { theme.muted() };
        let repo_style = if self.focused == 1 { theme.title() } else { theme.muted() };
        let cursor0 = if self.focused == 0 { "█" } else { "" };
        let cursor1 = if self.focused == 1 { "█" } else { "" };

        let token_display = if self.token_input.is_empty() {
            "ghp_xxxxxxxxxxxxxxxx".to_string()
        } else if self.token_input.len() > 20 {
            format!("{}...", &self.token_input[..20])
        } else {
            self.token_input.clone()
        };

        let form = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled("  GitHub Token:", token_style)),
            Line::from(Span::styled("  Necessário para baixar binários do repo privado.", theme.dim())),
            Line::from(vec![
                Span::styled("  ", theme.normal()),
                Span::styled(token_display, if self.token_input.is_empty() { theme.dim() } else { theme.normal() }),
                Span::styled(cursor0, theme.normal()),
            ]),
            Line::from(""),
            Line::from(Span::styled("  Repositório:", repo_style)),
            Line::from(vec![
                Span::styled("  ", theme.normal()),
                Span::styled(&self.repo_input, theme.normal()),
                Span::styled(cursor1, theme.normal()),
            ]),
            Line::from(""),
        ]);
        frame.render_widget(form, chunks[1]);

        // Contextual help
        let help = Paragraph::new(vec![
            Line::from(Span::styled("  Como obter o token:", theme.muted())),
            Line::from(Span::styled("  1. Acesse github.com/settings/tokens → Generate new token", theme.dim())),
            Line::from(Span::styled("  2. Selecione scope: repo (Full control)", theme.dim())),
            Line::from(Span::styled("  3. Copie o token gerado (ghp_...) e cole aqui", theme.dim())),
            Line::from(""),
            Line::from(Span::styled("  Após configurar, você poderá instalar masters e agents", theme.muted())),
            Line::from(Span::styled("  diretamente por esta interface — sem comandos manuais.", theme.muted())),
        ])
        .wrap(Wrap { trim: false });
        frame.render_widget(help, chunks[2]);

        // Footer
        let footer = ui::footer_line(
            &[("Tab", "próximo campo"), ("Enter", "salvar e continuar"), ("Esc", "pular setup")],
            theme,
        );
        let fw = Paragraph::new(footer)
            .block(Block::default().borders(Borders::TOP).border_type(BORDER).border_style(theme.border()));
        frame.render_widget(fw, chunks[3]);
    }
}
