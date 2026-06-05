use ratatui::style::{Color, Modifier, Style};

pub struct Theme {
    pub bg: Color,
    pub fg: Color,
    pub accent: Color,
    pub muted: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub selected_bg: Color,
    pub border: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: Color::Reset,
            fg: Color::White,
            accent: Color::Cyan,
            muted: Color::DarkGray,
            success: Color::Green,
            warning: Color::Yellow,
            error: Color::Red,
            selected_bg: Color::Rgb(40, 40, 60),
            border: Color::Rgb(60, 60, 80),
        }
    }
}

impl Theme {
    pub fn header(&self) -> Style {
        Style::default().fg(self.accent).add_modifier(Modifier::BOLD)
    }

    pub fn normal(&self) -> Style {
        Style::default().fg(self.fg)
    }

    pub fn muted(&self) -> Style {
        Style::default().fg(self.muted)
    }

    pub fn selected(&self) -> Style {
        Style::default().bg(self.selected_bg).fg(self.fg).add_modifier(Modifier::BOLD)
    }

    pub fn online(&self) -> Style {
        Style::default().fg(self.success)
    }

    pub fn offline(&self) -> Style {
        Style::default().fg(self.error)
    }

    pub fn gauge_color(&self, pct: f64) -> Color {
        if pct > 90.0 {
            self.error
        } else if pct > 70.0 {
            self.warning
        } else {
            self.success
        }
    }

    pub fn footer(&self) -> Style {
        Style::default().fg(self.muted)
    }

    pub fn shortcut(&self) -> Style {
        Style::default().fg(self.accent)
    }
}
