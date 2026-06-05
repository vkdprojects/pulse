use crate::action::Action;
use crate::client::{AgentMetric, AgentOverview, Client, DiskInfo, NetInterface, ProcessInfo};
use crate::ui::{self, BORDER, theme::Theme};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    symbols,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph, Row, Table},
};

pub struct DetailScreen {
    pub agent_id: String,
    pub agent: Option<AgentOverview>,
    pub metrics: Vec<AgentMetric>,
    pub processes: Vec<ProcessInfo>,
    pub disks: Vec<DiskInfo>,
    pub network: Vec<NetInterface>,
    pub tab: usize,
}

impl DetailScreen {
    pub fn new(agent_id: String) -> Self {
        Self {
            agent_id,
            agent: None,
            metrics: Vec::new(),
            processes: Vec::new(),
            disks: Vec::new(),
            network: Vec::new(),
            tab: 0,
        }
    }

    pub async fn refresh(&mut self, client: &Client) {
        let id = &self.agent_id;
        let (metrics, procs, disks, net) = tokio::join!(
            client.agent_metrics(id),
            client.agent_processes(id),
            client.agent_disks(id),
            client.agent_network(id),
        );
        self.metrics = metrics.unwrap_or_default();
        self.processes = procs.unwrap_or_default();
        self.disks = disks.unwrap_or_default();
        self.network = net.unwrap_or_default();
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => return Action::NavigateFleet(String::new()),
            KeyCode::Tab => self.tab = (self.tab + 1) % 2,
            KeyCode::Char('1') => self.tab = 0,
            KeyCode::Char('2') => self.tab = 1,
            KeyCode::Char('q') => return Action::Quit,
            _ => {}
        }
        Action::None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::vertical([Constraint::Length(4), Constraint::Min(5), Constraint::Length(2)]).split(area);

        // Header
        let hostname = self.agent.as_ref().map(|a| a.hostname.as_str()).unwrap_or("...");
        let status = self.agent.as_ref().map(|a| a.status.as_str()).unwrap_or("?");
        let uptime = self.agent.as_ref().and_then(|a| a.uptime_secs).map(ui::format_uptime).unwrap_or_default();
        let version = self.agent.as_ref().map(|a| a.version.as_str()).unwrap_or("");
        let status_style = if status == "online" { theme.online() } else { theme.offline() };

        let tabs = ["Overview", "Processes"];
        let tab_spans: Vec<Span> = tabs
            .iter()
            .enumerate()
            .map(|(i, t)| {
                if i == self.tab {
                    Span::styled(format!(" [{t}] "), theme.title())
                } else {
                    Span::styled(format!("  {t}  "), theme.muted())
                }
            })
            .collect();

        let header = Paragraph::new(vec![
            Line::from(
                [
                    vec![
                        Span::styled(" ◉ pulse ", theme.title()),
                        Span::styled(hostname, theme.title()),
                        Span::styled("  ", theme.dim()),
                        Span::styled(if status == "online" { "●" } else { "○" }, status_style),
                        Span::styled(format!("  ↑{uptime}  {version}"), theme.muted()),
                        Span::styled("    ", theme.dim()),
                    ],
                    tab_spans,
                ]
                .concat(),
            ),
            Line::from(""),
        ])
        .block(Block::default().borders(Borders::BOTTOM).border_type(BORDER).border_style(theme.border()));
        frame.render_widget(header, chunks[0]);

        match self.tab {
            0 => self.render_overview(frame, chunks[1], theme),
            1 => self.render_processes(frame, chunks[1], theme),
            _ => {}
        }

        let footer = ui::footer_line(&[("Tab", "trocar"), ("e", "exec"), ("Esc", "voltar"), ("q", "sair")], theme);
        let footer_w = Paragraph::new(footer)
            .block(Block::default().borders(Borders::TOP).border_type(BORDER).border_style(theme.border()));
        frame.render_widget(footer_w, chunks[2]);
    }

    fn render_overview(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let layout = Layout::vertical([Constraint::Length(3), Constraint::Min(6), Constraint::Length(8)]).split(area);

        // Row 1: Stat cards
        let latest = self.metrics.first();
        let cpu = latest.and_then(|m| m.cpu_pct).unwrap_or(0.0);
        let mem_used = latest.and_then(|m| m.mem_used_mb).unwrap_or(0);
        let mem_total = latest.and_then(|m| m.mem_total_mb).unwrap_or(1);
        let mem_pct = if mem_total > 0 { mem_used as f64 * 100.0 / mem_total as f64 } else { 0.0 };
        let uptime_s = latest.and_then(|m| m.uptime_secs).unwrap_or(0);

        let stats = Layout::horizontal([
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ])
        .split(layout[0]);

        self.render_stat_card(frame, stats[0], "CPU", &format!("{cpu:.1}%"), theme.gauge_color(cpu), theme);
        self.render_stat_card(frame, stats[1], "MEM", &format!("{mem_pct:.1}%"), theme.gauge_color(mem_pct), theme);
        self.render_stat_card(frame, stats[2], "RAM", &format!("{}/{} MB", mem_used, mem_total), theme.mauve, theme);
        self.render_stat_card(frame, stats[3], "UPTIME", &ui::format_uptime(uptime_s), theme.blue, theme);
        self.render_stat_card(frame, stats[4], "DISKS", &format!("{}", self.disks.len()), theme.teal, theme);

        // Row 2: CPU + MEM charts side by side
        let charts = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)]).split(layout[1]);
        self.render_cpu_chart(frame, charts[0], theme);
        self.render_mem_chart(frame, charts[1], theme);

        // Row 3: Disks + Network
        let bottom = Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)]).split(layout[2]);
        self.render_disks_table(frame, bottom[0], theme);
        self.render_network_table(frame, bottom[1], theme);
    }

    fn render_stat_card(&self, frame: &mut Frame, area: Rect, label: &str, value: &str, color: Color, theme: &Theme) {
        let block = Block::default().borders(Borders::ALL).border_type(BORDER).border_style(theme.border());
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let p = Paragraph::new(vec![
            Line::from(Span::styled(value, Style::default().fg(color).add_modifier(ratatui::style::Modifier::BOLD))),
            Line::from(Span::styled(label, theme.dim())),
        ]);
        frame.render_widget(p, inner);
    }

    fn render_cpu_chart(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if self.metrics.len() < 2 {
            let empty = Paragraph::new(Span::styled("  Aguardando dados...", theme.muted())).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BORDER)
                    .border_style(theme.border())
                    .title(Span::styled(" CPU ", theme.muted())),
            );
            frame.render_widget(empty, area);
            return;
        }
        let data: Vec<(f64, f64)> =
            self.metrics.iter().rev().enumerate().filter_map(|(i, m)| m.cpu_pct.map(|v| (i as f64, v))).collect();
        let dataset = Dataset::default()
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .data(&data)
            .style(Style::default().fg(theme.peach));
        let max_x = data.len().max(1) as f64;
        let chart = Chart::new(vec![dataset])
            .x_axis(Axis::default().bounds([0.0, max_x]))
            .y_axis(Axis::default().bounds([0.0, 100.0]).labels(vec![Line::from("0"), Line::from("100")]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BORDER)
                    .border_style(theme.border())
                    .title(Span::styled(" CPU % ", theme.muted())),
            );
        frame.render_widget(chart, area);
    }

    fn render_mem_chart(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if self.metrics.len() < 2 {
            let empty = Paragraph::new(Span::styled("  Aguardando dados...", theme.muted())).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BORDER)
                    .border_style(theme.border())
                    .title(Span::styled(" MEM ", theme.muted())),
            );
            frame.render_widget(empty, area);
            return;
        }
        let data: Vec<(f64, f64)> = self
            .metrics
            .iter()
            .rev()
            .enumerate()
            .filter_map(|(i, m)| {
                let used = m.mem_used_mb? as f64;
                let total = m.mem_total_mb.filter(|t| *t > 0)? as f64;
                Some((i as f64, used / total * 100.0))
            })
            .collect();
        let dataset = Dataset::default()
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .data(&data)
            .style(Style::default().fg(theme.mauve));
        let max_x = data.len().max(1) as f64;
        let chart = Chart::new(vec![dataset])
            .x_axis(Axis::default().bounds([0.0, max_x]))
            .y_axis(Axis::default().bounds([0.0, 100.0]).labels(vec![Line::from("0"), Line::from("100")]))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BORDER)
                    .border_style(theme.border())
                    .title(Span::styled(" MEM % ", theme.muted())),
            );
        frame.render_widget(chart, area);
    }

    fn render_disks_table(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let rows: Vec<Row> = self
            .disks
            .iter()
            .map(|d| {
                let pct = d.used_pct.unwrap_or(0.0);
                let bar = ui::gauge_bar(pct, 10);
                Row::new(vec![
                    d.mount_point.clone(),
                    format!("{pct:.0}%"),
                    d.used_gb.map(|g| format!("{g:.1}G")).unwrap_or_default(),
                    bar,
                ])
            })
            .collect();
        let table = Table::new(
            rows,
            [Constraint::Min(8), Constraint::Length(4), Constraint::Length(5), Constraint::Length(12)],
        )
        .header(Row::new(vec!["MOUNT", "%", "USED", ""]).style(theme.dim()))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BORDER)
                .border_style(theme.border())
                .title(Span::styled(" Disks ", theme.muted())),
        );
        frame.render_widget(table, area);
    }

    fn render_network_table(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let rows: Vec<Row> = self
            .network
            .iter()
            .map(|n| {
                Row::new(vec![
                    n.name.clone(),
                    format!("↓{}", format_bytes(n.rx_bytes.unwrap_or(0))),
                    format!("↑{}", format_bytes(n.tx_bytes.unwrap_or(0))),
                ])
            })
            .collect();
        let table = Table::new(rows, [Constraint::Min(8), Constraint::Length(10), Constraint::Length(10)])
            .header(Row::new(vec!["IFACE", "RX/s", "TX/s"]).style(theme.dim()))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BORDER)
                    .border_style(theme.border())
                    .title(Span::styled(" Network ", theme.muted())),
            );
        frame.render_widget(table, area);
    }

    fn render_processes(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let rows: Vec<Row> = self
            .processes
            .iter()
            .map(|p| {
                Row::new(vec![
                    format!("{:>6}", p.pid),
                    format!("{:<20}", p.name),
                    p.cpu_pct.map(|v| format!("{v:.1}%")).unwrap_or_default(),
                    p.rss_mb.map(|v| format!("{v} MB")).unwrap_or_default(),
                ])
            })
            .collect();
        let table = Table::new(
            rows,
            [Constraint::Length(7), Constraint::Min(20), Constraint::Length(8), Constraint::Length(10)],
        )
        .header(Row::new(vec!["PID", "NAME", "CPU", "RSS"]).style(theme.dim()))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BORDER)
                .border_style(theme.border())
                .title(Span::styled(" Top Processes ", theme.muted())),
        );
        frame.render_widget(table, area);
    }
}

fn format_bytes(b: i64) -> String {
    if b > 1_000_000 {
        format!("{:.1}M", b as f64 / 1_000_000.0)
    } else if b > 1_000 {
        format!("{:.0}K", b as f64 / 1_000.0)
    } else {
        format!("{b}B")
    }
}

use ratatui::style::Color;
