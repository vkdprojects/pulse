pub mod modal;
pub mod theme;

use ratatui::text::{Line, Span};
use theme::Theme;

pub fn footer_line<'a>(keys: &[(&'a str, &'a str)], theme: &Theme) -> Line<'a> {
    let mut spans = Vec::new();
    for (i, (key, desc)) in keys.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", theme.muted()));
        }
        spans.push(Span::styled(format!("[{key}]"), theme.shortcut()));
        spans.push(Span::styled(format!(" {desc}"), theme.muted()));
    }
    Line::from(spans)
}

pub fn format_uptime(secs: i64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    if days > 0 {
        format!("{days}d {hours}h")
    } else {
        let mins = (secs % 3600) / 60;
        format!("{hours}h {mins}m")
    }
}

pub fn gauge_bar(pct: f64, width: usize) -> String {
    let filled = (pct / 100.0 * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}
