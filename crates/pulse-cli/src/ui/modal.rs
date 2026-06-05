use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

pub fn centered_rect(pct_x: u16, pct_y: u16, area: Rect) -> Rect {
    let w = area.width * pct_x / 100;
    let h = area.height * pct_y / 100;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect::new(x, y, w, h)
}

pub fn render_modal(frame: &mut Frame, title: &str, lines: &[&str], area: Rect) {
    let popup = centered_rect(60, 50, area);
    frame.render_widget(Clear, popup);
    let text = lines.join("\n");
    let widget = Paragraph::new(text).wrap(Wrap { trim: false }).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(format!(" {} ", title)),
    );
    frame.render_widget(widget, popup);
}
