use crate::app::{App, DetailTab, Screen};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Tabs},
};

pub fn render(frame: &mut Frame, app: &App) {
    match app.screen {
        Screen::Fleet => render_fleet(frame, app),
        Screen::Detail => render_detail(frame, app),
        Screen::Update => render_update(frame, app),
        Screen::Exec => render_exec(frame, app),
    }
}

fn render_fleet(frame: &mut Frame, app: &App) {
    let chunks =
        Layout::vertical([Constraint::Length(3), Constraint::Min(5), Constraint::Length(1)]).split(frame.area());

    // Header
    let online = app.agents.iter().filter(|a| a.status == "online").count();
    let total = app.agents.len();
    let header_text =
        format!("⚡ Pulse Fleet  │  {} agents  │  {} online  │  {} offline", total, online, total - online);
    let header =
        Paragraph::new(header_text).style(Style::default().bold()).block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(header, chunks[0]);

    // Agents table
    let filtered = app.filtered_agents();
    let rows: Vec<Row> = filtered
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let status_style = match a.status.as_str() {
                "online" => Style::default().fg(Color::Green),
                _ => Style::default().fg(Color::Red),
            };
            let status_icon = if a.status == "online" { "●" } else { "○" };
            let cpu = a.cpu_pct.map(|v| format!("{:.0}%", v)).unwrap_or_else(|| "—".into());
            let mem = a.mem_pct.map(|v| format!("{:.0}%", v)).unwrap_or_else(|| "—".into());
            let uptime = a.uptime_secs.map(format_uptime).unwrap_or_else(|| "—".into());

            let style = if i == app.selected {
                Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(Span::styled(status_icon, status_style)),
                Cell::from(a.hostname.as_str()),
                Cell::from(cpu),
                Cell::from(mem),
                Cell::from(uptime),
                Cell::from(a.id.as_str()),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(2),
            Constraint::Min(16),
            Constraint::Length(6),
            Constraint::Length(6),
            Constraint::Length(10),
            Constraint::Length(12),
        ],
    )
    .header(
        Row::new(vec!["", "HOSTNAME", "CPU", "MEM", "UPTIME", "ID"])
            .style(Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)),
    )
    .block(Block::default().borders(Borders::NONE));

    frame.render_widget(table, chunks[1]);

    // Footer
    let footer_text = if app.filtering {
        format!("filter: {}█", app.filter)
    } else if let Some(ref e) = app.error {
        format!("⚠ {}", e)
    } else {
        "[j/k] navigate  [Enter] detail  [/] filter  [q] quit".into()
    };
    let footer = Paragraph::new(footer_text).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, chunks[2]);
}

fn render_detail(frame: &mut Frame, app: &App) {
    let chunks =
        Layout::vertical([Constraint::Length(3), Constraint::Min(5), Constraint::Length(1)]).split(frame.area());

    // Header with tabs
    let agent_name = app.selected_agent().map(|a| a.hostname.as_str()).unwrap_or("?");
    let tabs = Tabs::new(vec!["System", "Processes"])
        .select(app.detail_tab as usize)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(Style::default().fg(Color::Cyan).bold())
        .block(Block::default().borders(Borders::BOTTOM).title(format!("⚡ {agent_name}")));
    frame.render_widget(tabs, chunks[0]);

    // Content
    match app.detail_tab {
        DetailTab::System => render_system_tab(frame, app, chunks[1]),
        DetailTab::Processes => render_processes_tab(frame, app, chunks[1]),
    }

    // Footer
    let footer =
        Paragraph::new("[Esc] back  [Tab] switch  [1/2] tabs  [q] quit").style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, chunks[2]);
}

fn render_system_tab(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::vertical([Constraint::Length(6), Constraint::Min(3)]).split(area);

    // Metrics gauges
    let latest = app.metrics.first();
    let cpu = latest.and_then(|m| m.cpu_pct).unwrap_or(0.0);
    let mem_used = latest.and_then(|m| m.mem_used_mb).unwrap_or(0);
    let mem_total = latest.and_then(|m| m.mem_total_mb).unwrap_or(1);
    let mem_pct = if mem_total > 0 { mem_used as f64 * 100.0 / mem_total as f64 } else { 0.0 };
    let uptime = latest.and_then(|m| m.uptime_secs).map(format_uptime).unwrap_or_default();

    let gauge_lines = vec![
        Line::from(vec![
            Span::raw(" CPU  "),
            Span::styled(gauge_bar(cpu, 30), color_for_pct(cpu)),
            Span::raw(format!("  {cpu:.1}%")),
        ]),
        Line::from(vec![
            Span::raw(" MEM  "),
            Span::styled(gauge_bar(mem_pct, 30), color_for_pct(mem_pct)),
            Span::raw(format!("  {:.1}%  ({}/{} MB)", mem_pct, mem_used, mem_total)),
        ]),
        Line::from(""),
        Line::from(format!(" Uptime: {uptime}")),
    ];
    let gauges = Paragraph::new(gauge_lines).block(Block::default().borders(Borders::NONE));
    frame.render_widget(gauges, chunks[0]);

    // Disks + Network
    let sub = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(chunks[1]);

    let disk_rows: Vec<Row> = app
        .disks
        .iter()
        .map(|d| {
            Row::new(vec![
                Cell::from(d.mount_point.as_str()),
                Cell::from(d.used_pct.map(|p| format!("{p:.0}%")).unwrap_or_default()),
                Cell::from(d.used_gb.map(|g| format!("{g:.1}")).unwrap_or_default()),
                Cell::from(d.total_gb.map(|g| format!("{g:.1}")).unwrap_or_default()),
            ])
        })
        .collect();
    let disk_table = Table::new(
        disk_rows,
        [Constraint::Min(8), Constraint::Length(5), Constraint::Length(6), Constraint::Length(6)],
    )
    .header(Row::new(vec!["MOUNT", "USE%", "USED", "TOTAL"]).style(Style::default().fg(Color::DarkGray)))
    .block(Block::default().borders(Borders::ALL).title("Disks"));
    frame.render_widget(disk_table, sub[0]);

    let net_rows: Vec<Row> = app
        .network
        .iter()
        .map(|n| {
            Row::new(vec![
                Cell::from(n.name.as_str()),
                Cell::from(format_bytes(n.rx_bytes.unwrap_or(0))),
                Cell::from(format_bytes(n.tx_bytes.unwrap_or(0))),
            ])
        })
        .collect();
    let net_table = Table::new(net_rows, [Constraint::Min(8), Constraint::Length(10), Constraint::Length(10)])
        .header(Row::new(vec!["IFACE", "RX/s", "TX/s"]).style(Style::default().fg(Color::DarkGray)))
        .block(Block::default().borders(Borders::ALL).title("Network"));
    frame.render_widget(net_table, sub[1]);
}

fn render_processes_tab(frame: &mut Frame, app: &App, area: Rect) {
    let rows: Vec<Row> = app
        .processes
        .iter()
        .map(|p| {
            Row::new(vec![
                Cell::from(p.pid.to_string()),
                Cell::from(p.name.as_str()),
                Cell::from(p.cpu_pct.map(|v| format!("{v:.1}%")).unwrap_or_default()),
                Cell::from(p.rss_mb.map(|v| format!("{v} MB")).unwrap_or_default()),
            ])
        })
        .collect();

    let table =
        Table::new(rows, [Constraint::Length(7), Constraint::Min(16), Constraint::Length(8), Constraint::Length(10)])
            .header(Row::new(vec!["PID", "NAME", "CPU", "RSS"]).style(Style::default().fg(Color::DarkGray).bold()))
            .block(Block::default().borders(Borders::ALL).title("Top Processes"));
    frame.render_widget(table, area);
}

// ─── Helpers ──────────────────────────────────────────────────────

fn render_update(frame: &mut Frame, app: &App) {
    let chunks =
        Layout::vertical([Constraint::Length(3), Constraint::Min(5), Constraint::Length(1)]).split(frame.area());

    let target_str = if app.target_version.is_empty() { "none" } else { &app.target_version };
    let header =
        Paragraph::new(format!("⚡ Fleet Update  │  Target: {}  │  Rollout: {}%", target_str, app.rollout_pct))
            .style(Style::default().add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(header, chunks[0]);

    let rows: Vec<Row> = app
        .agents
        .iter()
        .map(|a| {
            let status_icon = if a.version == app.target_version || app.target_version.is_empty() {
                "✅"
            } else if a.status == "offline" {
                "❌"
            } else {
                "⏳"
            };
            let version_style = if a.version == app.target_version || app.target_version.is_empty() {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Yellow)
            };
            Row::new(vec![
                Cell::from(a.hostname.as_str()),
                Cell::from(Span::styled(a.version.as_str(), version_style)),
                Cell::from(target_str),
                Cell::from(status_icon),
            ])
        })
        .collect();

    let table =
        Table::new(rows, [Constraint::Min(16), Constraint::Length(10), Constraint::Length(10), Constraint::Length(4)])
            .header(
                Row::new(vec!["HOSTNAME", "CURRENT", "TARGET", ""])
                    .style(Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD)),
            )
            .block(Block::default().borders(Borders::NONE));
    frame.render_widget(table, chunks[1]);

    let footer = Paragraph::new("[Esc] back  [q] quit").style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, chunks[2]);
}

fn render_exec(frame: &mut Frame, app: &App) {
    let chunks =
        Layout::vertical([Constraint::Length(3), Constraint::Length(3), Constraint::Min(5), Constraint::Length(1)])
            .split(frame.area());

    let agent_name = app.selected_agent().map(|a| a.hostname.as_str()).unwrap_or("?");
    let header = Paragraph::new(format!("⚡ Exec @ {agent_name}"))
        .style(Style::default().add_modifier(Modifier::BOLD))
        .block(Block::default().borders(Borders::BOTTOM));
    frame.render_widget(header, chunks[0]);

    let input =
        Paragraph::new(format!("$ {}█", app.exec_input)).block(Block::default().borders(Borders::ALL).title("Command"));
    frame.render_widget(input, chunks[1]);

    let output = Paragraph::new(app.exec_output.as_str()).block(Block::default().borders(Borders::ALL).title("Output"));
    frame.render_widget(output, chunks[2]);

    let footer = Paragraph::new("[Enter] run  [Esc] back  [q] quit").style(Style::default().fg(Color::DarkGray));
    frame.render_widget(footer, chunks[3]);
}

// ─── Utility ──────────────────────────────────────────────────────

fn gauge_bar(pct: f64, width: usize) -> String {
    let filled = (pct / 100.0 * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

fn color_for_pct(pct: f64) -> Style {
    if pct > 90.0 {
        Style::default().fg(Color::Red)
    } else if pct > 70.0 {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default().fg(Color::Green)
    }
}

fn format_uptime(secs: i64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    if days > 0 {
        format!("{days}d {hours}h")
    } else {
        let mins = (secs % 3600) / 60;
        format!("{hours}h {mins}m")
    }
}

fn format_bytes(b: i64) -> String {
    if b > 1_000_000_000 {
        format!("{:.1} GB", b as f64 / 1_000_000_000.0)
    } else if b > 1_000_000 {
        format!("{:.1} MB", b as f64 / 1_000_000.0)
    } else if b > 1_000 {
        format!("{:.1} KB", b as f64 / 1_000.0)
    } else {
        format!("{b} B")
    }
}
