use ratatui::style::{Color, Modifier, Style};

/// Catppuccin Mocha color palette
pub struct Theme {
    // Backgrounds
    pub base: Color,
    pub surface0: Color,
    pub surface1: Color,
    pub surface2: Color,

    // Foregrounds
    pub text: Color,
    pub subtext: Color,
    pub overlay: Color,

    // Accents
    pub blue: Color,
    pub green: Color,
    pub red: Color,
    pub yellow: Color,
    pub peach: Color,
    pub mauve: Color,
    pub teal: Color,
    pub lavender: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            // Catppuccin Mocha
            base: Color::Rgb(30, 30, 46),
            surface0: Color::Rgb(49, 50, 68),
            surface1: Color::Rgb(69, 71, 90),
            surface2: Color::Rgb(88, 91, 112),

            text: Color::Rgb(205, 214, 244),
            subtext: Color::Rgb(166, 173, 200),
            overlay: Color::Rgb(108, 112, 134),

            blue: Color::Rgb(137, 180, 250),
            green: Color::Rgb(166, 227, 161),
            red: Color::Rgb(243, 139, 168),
            yellow: Color::Rgb(249, 226, 175),
            peach: Color::Rgb(250, 179, 135),
            mauve: Color::Rgb(203, 166, 247),
            teal: Color::Rgb(148, 226, 213),
            lavender: Color::Rgb(180, 190, 254),
        }
    }
}

impl Theme {
    pub fn title(&self) -> Style {
        Style::default().fg(self.lavender).add_modifier(Modifier::BOLD)
    }

    pub fn normal(&self) -> Style {
        Style::default().fg(self.text)
    }

    pub fn muted(&self) -> Style {
        Style::default().fg(self.overlay)
    }

    pub fn dim(&self) -> Style {
        Style::default().fg(self.surface2)
    }

    pub fn selected(&self) -> Style {
        Style::default().bg(self.surface1).fg(self.text).add_modifier(Modifier::BOLD)
    }

    pub fn online(&self) -> Style {
        Style::default().fg(self.green)
    }

    pub fn offline(&self) -> Style {
        Style::default().fg(self.red)
    }

    pub fn warning(&self) -> Style {
        Style::default().fg(self.yellow)
    }

    pub fn border(&self) -> Style {
        Style::default().fg(self.surface2)
    }

    pub fn border_focused(&self) -> Style {
        Style::default().fg(self.blue)
    }

    pub fn gauge_color(&self, pct: f64) -> Color {
        if pct > 90.0 {
            self.red
        } else if pct > 70.0 {
            self.yellow
        } else {
            self.green
        }
    }

    pub fn key_style(&self) -> Style {
        Style::default().fg(self.blue).add_modifier(Modifier::BOLD)
    }

    pub fn desc_style(&self) -> Style {
        Style::default().fg(self.overlay)
    }

    pub fn header_bg(&self) -> Style {
        Style::default().bg(self.surface0).fg(self.text)
    }
}
