pub mod modal;
pub mod theme;

use ratatui::{
    text::{Line, Span},
    widgets::BorderType,
};
use theme::Theme;

/// Rounded border type for all panels
pub const BORDER: BorderType = BorderType::Rounded;

/// Pulse logo ASCII art (compact, fits in ~6 lines)
pub const LOGO: &[&str] = &["  ┏━┓╻ ╻╻  ┏━┓┏━╸", "  ┣━┛┃ ┃┃  ┗━┓┣╸ ", "  ╹  ┗━┛┗━╸┗━┛┗━╸"];

/// Colored logo lines
pub fn logo_lines(theme: &Theme) -> Vec<Line<'static>> {
    LOGO.iter().map(|line| Line::from(Span::styled(line.to_string(), theme.title()))).collect()
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_uptime_days() {
        assert_eq!(format_uptime(90000), "1d1h"); // 1 day + 1 hour
        assert_eq!(format_uptime(86400), "1d0h");
        assert_eq!(format_uptime(172800), "2d0h");
    }

    #[test]
    fn test_format_uptime_hours() {
        assert_eq!(format_uptime(3600), "1h0m");
        assert_eq!(format_uptime(7200), "2h0m");
        assert_eq!(format_uptime(5400), "1h30m");
    }

    #[test]
    fn test_format_uptime_minutes() {
        assert_eq!(format_uptime(60), "0h1m");
        assert_eq!(format_uptime(0), "0h0m");
    }

    #[test]
    fn test_gauge_bar() {
        let bar = gauge_bar(50.0, 10);
        assert_eq!(bar.chars().filter(|c| *c == '█').count(), 5);
        assert_eq!(bar.chars().filter(|c| *c == '░').count(), 5);
    }

    #[test]
    fn test_gauge_bar_empty() {
        let bar = gauge_bar(0.0, 10);
        assert_eq!(bar.chars().filter(|c| *c == '░').count(), 10);
    }

    #[test]
    fn test_gauge_bar_full() {
        let bar = gauge_bar(100.0, 10);
        assert_eq!(bar.chars().filter(|c| *c == '█').count(), 10);
    }

    #[test]
    fn test_sparkline_str_empty() {
        let result = sparkline_str(&[], 10);
        assert_eq!(result, "──────────");
    }

    #[test]
    fn test_sparkline_str_data() {
        let data = vec![0, 25, 50, 75, 100];
        let result = sparkline_str(&data, 5);
        assert_eq!(result.chars().count(), 5);
        // First char should be lowest (▁), last should be highest (█)
        let chars: Vec<char> = result.chars().collect();
        assert_eq!(chars[0], '▁');
        assert_eq!(chars[4], '█');
    }

    #[test]
    fn test_sparkline_str_truncates() {
        let data = vec![50; 20]; // 20 points but width=5
        let result = sparkline_str(&data, 5);
        assert_eq!(result.chars().count(), 5);
    }
}
