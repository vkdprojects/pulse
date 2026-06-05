use crate::action::Action;
use crate::client::{AgentMetric, AgentOverview, Client, DiskInfo, ProcessInfo};
use crate::ui::{self, theme::Theme};
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
    pub tab: usize,
}

impl DetailScreen {
    pub fn new(agent_id: String) -> Self {
        Self { agent_id, agent: None, metrics: Vec::new(), processes: Vec::new(), disks: Vec::new(), tab: 0 }
    }

    pub async fn refresh(&mut self, client: &Client) {
        let id = &self.agent_id;
        let (metrics, procs, disks) =
            tokio::join!(client.agent_metrics(id), client.agent_processes(id), client.agent_disks(id),);
        self.metrics = metrics.unwrap_or_default();
        self.processes = procs.unwrap_or_default();
        self.disks = disks.unwrap_or_default();
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => return Action::NavigateFleet(String::new()),
            KeyCode::Tab => self.tab = (self.tab + 1) % 3,
            KeyCode::Char('1') => self.tab = 0,
            KeyCode::Char('2') => self.tab = 1,
            KeyCode::Char('3') => self.tab = 2,
            KeyCode::Char('q') => return Action::Quit,
            _ => {}
        }
        Action::None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(5), Constraint::Length(1)]).split(area);

        // Header with tabs
        let hostname = self.agent.as_ref().map(|a| a.hostname.as_str()).unwrap_or("...");
        let tabs = ["System", "Processes", "Disks"];
        let tab_spans: Vec<Span> = tabs
            .iter()
            .enumerate()
            .map(|(i, t)| {
                if i == self.tab {
                    Span::styled(format!(" [{t}] "), theme.header())
                } else {
                    Span::styled(format!("  {t}  "), theme.muted())
                }
            })
            .collect();

        let header = Paragraph::new(Line::from(
            [vec![Span::styled(format!("⚡ {hostname}  "), theme.header())], tab_spans].concat(),
        ))
        .block(Block::default().borders(Borders::BOTTOM).border_style(theme.muted()));
        frame.render_widget(header, chunks[0]);

        // Content
        match self.tab {
            0 => self.render_system(frame, chunks[1], theme),
            1 => self.render_processes(frame, chunks[1], theme),
            2 => self.render_disks(frame, chunks[1], theme),
            _ => {}
        }

        // Footer
        let footer = ui::footer_line(&[("Tab", "trocar"), ("1-3", "tabs"), ("Esc", "voltar"), ("q", "sair")], theme);
        frame.render_widget(Paragraph::new(footer), chunks[2]);
    }

    fn render_system(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let chunks = Layout::vertical([Constraint::Length(5), Constraint::Min(6)]).split(area);

        // CPU + MEM gauges
        let latest = self.metrics.first();
        let cpu = latest.and_then(|m| m.cpu_pct).unwrap_or(0.0);
        let mem_used = latest.and_then(|m| m.mem_used_mb).unwrap_or(0);
        let mem_total = latest.and_then(|m| m.mem_total_mb).unwrap_or(1);
        let mem_pct = if mem_total > 0 { mem_used as f64 * 100.0 / mem_total as f64 } else { 0.0 };
        let uptime = latest.and_then(|m| m.uptime_secs).map(ui::format_uptime).unwrap_or_default();

        let cpu_bar = ui::gauge_bar(cpu, 30);
        let mem_bar = ui::gauge_bar(mem_pct, 30);

        let gauges = Paragraph::new(vec![
            Line::from(vec![
                Span::styled(" CPU  ", theme.muted()),
                Span::styled(&cpu_bar, Style::default().fg(theme.gauge_color(cpu))),
                Span::styled(format!("  {cpu:.1}%"), theme.normal()),
            ]),
            Line::from(vec![
                Span::styled(" MEM  ", theme.muted()),
                Span::styled(&mem_bar, Style::default().fg(theme.gauge_color(mem_pct))),
                Span::styled(format!("  {mem_pct:.1}%  ({mem_used}/{mem_total} MB)"), theme.normal()),
            ]),
            Line::from(""),
            Line::from(Span::styled(format!(" Uptime: {uptime}"), theme.muted())),
        ]);
        frame.render_widget(gauges, chunks[0]);

        // CPU chart (historical)
        if self.metrics.len() >= 2 {
            let data: Vec<(f64, f64)> =
                self.metrics.iter().rev().enumerate().filter_map(|(i, m)| m.cpu_pct.map(|v| (i as f64, v))).collect();

            let dataset = Dataset::default()
                .marker(symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .data(&data)
                .style(Style::default().fg(theme.accent));

            let max_x = data.len() as f64;
            let chart = Chart::new(vec![dataset])
                .x_axis(Axis::default().bounds([0.0, max_x]))
                .y_axis(Axis::default().bounds([0.0, 100.0]).labels(vec![Line::from("0%"), Line::from("100%")]))
                .block(Block::default().borders(Borders::ALL).border_style(theme.muted()).title(" CPU History "));
            frame.render_widget(chart, chunks[1]);
        }
    }

    fn render_processes(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let rows: Vec<Row> = self
            .processes
            .iter()
            .map(|p| {
                Row::new(vec![
                    p.pid.to_string(),
                    p.name.clone(),
                    p.cpu_pct.map(|v| format!("{v:.1}%")).unwrap_or_default(),
                    p.rss_mb.map(|v| format!("{v} MB")).unwrap_or_default(),
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            [Constraint::Length(7), Constraint::Min(16), Constraint::Length(8), Constraint::Length(10)],
        )
        .header(Row::new(vec!["PID", "NAME", "CPU", "RSS"]).style(theme.muted()))
        .block(Block::default().borders(Borders::ALL).border_style(theme.muted()).title(" Top Processes "));
        frame.render_widget(table, area);
    }

    fn render_disks(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let rows: Vec<Row> = self
            .disks
            .iter()
            .map(|d| {
                let pct = d.used_pct.unwrap_or(0.0);
                Row::new(vec![
                    d.mount_point.clone(),
                    d.used_pct.map(|p| format!("{p:.0}%")).unwrap_or_default(),
                    d.used_gb.map(|g| format!("{g:.1} GB")).unwrap_or_default(),
                    d.total_gb.map(|g| format!("{g:.1} GB")).unwrap_or_default(),
                    ui::gauge_bar(pct, 15),
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Min(10),
                Constraint::Length(5),
                Constraint::Length(8),
                Constraint::Length(8),
                Constraint::Length(17),
            ],
        )
        .header(Row::new(vec!["MOUNT", "USE%", "USED", "TOTAL", ""]).style(theme.muted()))
        .block(Block::default().borders(Borders::ALL).border_style(theme.muted()).title(" Disks "));
        frame.render_widget(table, area);
    }
}
