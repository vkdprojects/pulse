pub mod modal;
pub mod theme;

use ratatui::{
    text::{Line, Span},
    widgets::BorderType,
};
use theme::Theme;

/// Rounded border type for all panels
pub const BORDER: BorderType = BorderType::Rounded;

/// Context-aware footer with colored keys
pub fn footer_line<'a>(keys: &[(&'a str, &'a str)], theme: &Theme) -> Line<'a> {
    let mut spans = Vec::new();
    for (i, (key, desc)) in keys.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" │ ", theme.dim()));
        }
        spans.push(Span::styled(format!("<{key}>"), theme.key_style()));
        spans.push(Span::styled(*desc, theme.desc_style()));
    }
    Line::from(spans)
}

/// Format uptime seconds to human-readable
pub fn format_uptime(secs: i64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    if days > 0 {
        format!("{days}d{hours}h")
    } else {
        let mins = (secs % 3600) / 60;
        format!("{hours}h{mins}m")
    }
}

/// Colored gauge bar using block characters
pub fn gauge_bar(pct: f64, width: usize) -> String {
    let filled = (pct / 100.0 * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

/// Sparkline from CPU history using block chars (▁▂▃▄▅▆▇█)
pub fn sparkline_str(data: &[u64], width: usize) -> String {
    const CHARS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    let points: Vec<&u64> = data.iter().rev().take(width).collect::<Vec<_>>().into_iter().rev().collect();
    if points.is_empty() {
        return "─".repeat(width);
    }
    let max = points.iter().copied().max().copied().unwrap_or(1).max(1);
    points
        .iter()
        .map(|&&v| {
            let idx = (v as f64 / max as f64 * 7.0).round() as usize;
            CHARS[idx.min(7)]
        })
        .collect()
}

/// Braille spinner frames for loading indicator
pub const SPINNER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
