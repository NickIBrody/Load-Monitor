use crate::app::{App, SortBy, Tab};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Gauge, List, ListItem, Paragraph, Row, Table, TableState, Tabs},
    Frame,
};
use std::time::Duration;

// ── Palette ──────────────────────────────────────────────────────────────────

const BG:      Color = Color::Rgb(14, 14, 24);
const BORDER:  Color = Color::Rgb(48, 48, 72);
const ACCENT:  Color = Color::Cyan;
const OK:      Color = Color::Green;
const WARN:    Color = Color::Yellow;
const DANGER:  Color = Color::Red;
const DIM:     Color = Color::Rgb(80, 80, 110);
const TEXT:    Color = Color::Rgb(220, 220, 235);
const HEADER:  Color = Color::Rgb(18, 18, 32);

// ── Entry point ──────────────────────────────────────────────────────────────

pub fn render(f: &mut Frame, app: &App) {
    let area = f.size();

    // Background fill
    f.render_widget(
        Block::default().style(Style::default().bg(BG)),
        area,
    );

    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // header (gauges)
            Constraint::Length(3), // tab bar
            Constraint::Min(0),    // content
            Constraint::Length(1), // status bar
        ])
        .split(area);

    render_header(f, app, root[0]);
    render_tabbar(f, app, root[1]);

    match app.current_tab {
        Tab::Monitor  => render_monitor(f, app, root[2]),
        Tab::Rules    => render_rules(f, app, root[2]),
        Tab::Settings => render_settings(f, app, root[2]),
        Tab::Log      => render_log(f, app, root[2]),
    }

    render_statusbar(f, app, root[3]);
}

// ── Header ───────────────────────────────────────────────────────────────────

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(style(BORDER))
        .title(Span::styled(
            "  ⚡  LOAD MONITOR  ",
            Style::default().fg(ACCENT).add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Center)
        .style(style(HEADER).bg(HEADER));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(38),
            Constraint::Percentage(38),
            Constraint::Percentage(24),
        ])
        .split(inner);

    if let Some(m) = &app.metrics {
        let cpu_pct = (m.cpu_total as u16).min(100);
        let cpu_col = threshold_color(cpu_pct as f32, 60.0, 80.0);
        let cpu_gauge = Gauge::default()
            .block(field_block("CPU"))
            .gauge_style(style(cpu_col).bg(Color::Rgb(28, 28, 46)))
            .percent(cpu_pct)
            .label(format!("{:.1}%", m.cpu_total));
        f.render_widget(cpu_gauge, cols[0]);

        let ram_pct = (if m.memory_total > 0 {
            (m.memory_used * 100 / m.memory_total) as u16
        } else { 0 }).min(100);
        let ram_col = threshold_color(ram_pct as f32, 65.0, 85.0);
        let ram_used_gb  = m.memory_used  as f64 / 1_073_741_824.0;
        let ram_total_gb = m.memory_total as f64 / 1_073_741_824.0;
        let ram_gauge = Gauge::default()
            .block(field_block("RAM"))
            .gauge_style(style(ram_col).bg(Color::Rgb(28, 28, 46)))
            .percent(ram_pct)
            .label(format!("{:.1} / {:.1} GB", ram_used_gb, ram_total_gb));
        f.render_widget(ram_gauge, cols[1]);

        let load_lines = vec![
            Line::from(vec![dim(" 1m "), load_span(m.load_average.one)]),
            Line::from(vec![dim(" 5m "), load_span(m.load_average.five)]),
            Line::from(vec![dim("15m "), load_span(m.load_average.fifteen)]),
        ];
        let load_para = Paragraph::new(load_lines)
            .block(field_block("Load avg"));
        f.render_widget(load_para, cols[2]);
    } else {
        let p = Paragraph::new(Span::styled("Collecting…", style(DIM)))
            .block(field_block("System"));
        f.render_widget(p, inner);
    }
}

// ── Tab bar ──────────────────────────────────────────────────────────────────

fn render_tabbar(f: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = vec![
        Line::from(" [1] Monitor  "),
        Line::from(" [2] Rules    "),
        Line::from(" [3] Settings "),
        Line::from(" [4] Log      "),
    ];

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(style(BORDER))
                .title(Span::styled(
                    format!("  {} procs  interval {}s  ",
                        app.processes.len(),
                        app.config.general.interval_secs,
                    ),
                    style(DIM),
                ))
                .title_alignment(Alignment::Right),
        )
        .select(app.current_tab.index())
        .style(style(DIM))
        .highlight_style(
            Style::default()
                .fg(ACCENT)
                .bg(Color::Rgb(28, 28, 52))
                .add_modifier(Modifier::BOLD),
        )
        .divider(Span::styled("│", style(BORDER)));

    f.render_widget(tabs, area);
}

// ── Monitor tab ──────────────────────────────────────────────────────────────

fn render_monitor(f: &mut Frame, app: &App, area: Rect) {
    let sort_label = app.sort_by.label();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(style(BORDER))
        .title(Span::styled(
            format!("  Processes   [s] sort: {}  [r] reload config  [↑↓/jk] navigate ", sort_label),
            style(ACCENT).add_modifier(Modifier::BOLD),
        ));

    let header = Row::new(vec![
        Cell::from("   PID").style(hdr()),
        Cell::from("Name").style(hdr()),
        Cell::from(" CPU%").style(hdr()),
        Cell::from("   RAM").style(hdr()),
        Cell::from("Service").style(hdr()),
        Cell::from("Status").style(hdr()),
    ])
    .height(1)
    .bottom_margin(1);

    let bl = &app.config.limits.blacklist;

    let rows: Vec<Row> = app.processes.iter().map(|p| {
        let cpu_col = threshold_color(p.cpu_usage, 50.0, 80.0);
        let mem_str = fmt_bytes(p.memory);
        let is_bl   = bl.iter().any(|b| p.name.contains(b.as_str()));
        let status  = if is_bl { Span::styled("protected", style(Color::Blue)) }
                      else if p.cpu_usage > 80.0 { Span::styled("⚠ high cpu", style(DANGER)) }
                      else { Span::styled("normal", style(Color::Rgb(50, 90, 50))) };

        let name = truncate(&p.name, 28);
        let svc  = p.systemd_service.as_deref().unwrap_or("—");
        let svc  = truncate(svc, 20);

        Row::new(vec![
            Cell::from(format!("{:>7}", p.pid)).style(style(DIM)),
            Cell::from(name).style(style(TEXT)),
            Cell::from(format!("{:>5.1}%", p.cpu_usage)).style(style(cpu_col)),
            Cell::from(format!("{:>6}", mem_str)).style(style(ACCENT)),
            Cell::from(svc).style(style(DIM)),
            Cell::from(status),
        ])
    }).collect();

    let mut state = TableState::default();
    state.select(Some(app.selected_process.min(app.processes.len().saturating_sub(1))));

    let table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Min(28),
            Constraint::Length(7),
            Constraint::Length(8),
            Constraint::Length(22),
            Constraint::Min(12),
        ],
    )
    .header(header)
    .block(block)
    .row_highlight_style(
        Style::default()
            .bg(Color::Rgb(32, 32, 58))
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("▶ ");

    f.render_stateful_widget(table, area, &mut state);
}

// ── Rules tab ────────────────────────────────────────────────────────────────

fn render_rules(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(style(BORDER))
        .title(Span::styled("  Rules ", style(ACCENT).add_modifier(Modifier::BOLD)));

    if app.config.rules.is_empty() {
        let p = Paragraph::new(Span::styled("No rules configured.", style(DIM)))
            .block(block)
            .alignment(Alignment::Center);
        f.render_widget(p, area);
        return;
    }

    let items: Vec<ListItem> = app.config.rules.iter().map(|r| {
        let cond = format!("{:?}", r.condition);
        let act  = format!("{:?}", r.action);
        ListItem::new(vec![
            Line::from(vec![
                Span::styled("  ▸ ", style(ACCENT)),
                Span::styled(r.name.clone(), style(TEXT).add_modifier(Modifier::BOLD)),
                Span::styled(format!("  (triggers after {}s)", r.duration_secs), style(DIM)),
            ]),
            Line::from(vec![
                Span::styled("      if  ", style(DIM)),
                Span::styled(cond, style(WARN)),
            ]),
            Line::from(vec![
                Span::styled("      do  ", style(DIM)),
                Span::styled(act, style(OK)),
            ]),
            Line::from(""),
        ])
    }).collect();

    let list = List::new(items).block(block).style(style(BG).bg(BG));
    f.render_widget(list, area);
}

// ── Settings tab ─────────────────────────────────────────────────────────────

fn render_settings(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(style(BORDER))
        .title(Span::styled(
            "  Settings   [↑↓/jk] navigate   [Enter/e] edit   [w] save  ",
            style(ACCENT).add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // 4 fields × (3 rows field + 1 row gap) + footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(3),
        ])
        .split(inner);

    let fields: [(&str, String); 4] = [
        ("Check interval (seconds)",        app.config.general.interval_secs.to_string()),
        ("Action history size",             app.config.general.history_size.to_string()),
        ("Default CPU quota (%)",           format!("{:.1}", app.config.limits.default_cpu_quota)),
        ("Default memory limit (bytes)",    app.config.limits.default_memory_limit.to_string()),
    ];

    let field_slots = [chunks[0], chunks[2], chunks[4], chunks[6]];

    for (i, ((label, value), slot)) in fields.iter().zip(field_slots.iter()).enumerate() {
        let selected = app.settings_selected == i;
        let editing  = selected && app.settings_editing;

        let border_col = if editing { OK } else if selected { ACCENT } else { BORDER };
        let label_col  = if selected { TEXT } else { DIM };
        let val_col    = if editing { OK } else if selected { ACCENT } else { Color::Rgb(170, 170, 200) };

        let display = if editing {
            format!("{}█", app.settings_edit_buf)
        } else {
            value.clone()
        };

        let para = Paragraph::new(Span::styled(display, style(val_col)))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(style(border_col))
                    .title(Span::styled(format!(" {} ", label), style(label_col))),
            );

        f.render_widget(para, *slot);
    }

    // Footer: blacklist
    let bl = if app.config.limits.blacklist.is_empty() {
        "none".to_string()
    } else {
        app.config.limits.blacklist.join(", ")
    };
    let footer = Paragraph::new(vec![
        Line::from(vec![dim("Protected (blacklist): "), Span::styled(bl, style(WARN))]),
        Line::from(vec![dim("Whitelist: "), Span::styled(
            if app.config.limits.whitelist.is_empty() { "all processes".to_string() }
            else { app.config.limits.whitelist.join(", ") },
            style(OK),
        )]),
    ])
    .block(Block::default().borders(Borders::ALL).border_style(style(BORDER)).title(" Config info "));
    f.render_widget(footer, chunks[7]);
}

// ── Log tab ──────────────────────────────────────────────────────────────────

fn render_log(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(style(BORDER))
        .title(Span::styled(
            format!("  Action Log   {} entries   [↑↓/jk] scroll   [c] clear  ",
                app.action_log.len()),
            style(ACCENT).add_modifier(Modifier::BOLD),
        ));

    if app.action_log.is_empty() {
        let p = Paragraph::new(Span::styled("No actions yet.", style(DIM)))
            .block(block)
            .alignment(Alignment::Center);
        f.render_widget(p, area);
        return;
    }

    let items: Vec<ListItem> = app.action_log.iter().rev()
        .skip(app.log_scroll)
        .map(|e| {
            let col   = if e.success { OK } else { DANGER };
            let icon  = if e.success { "✓" } else { "✗" };
            ListItem::new(Line::from(vec![
                Span::styled(e.timestamp.format("  %H:%M:%S  ").to_string(), style(DIM)),
                Span::styled(icon, style(col).add_modifier(Modifier::BOLD)),
                Span::styled(format!("  {:>6}  ", e.pid), style(DIM)),
                Span::styled(truncate(&e.name, 22), style(TEXT)),
                Span::styled("  →  ", style(DIM)),
                Span::styled(e.action.clone(), style(col)),
            ]))
        })
        .collect();

    let list = List::new(items).block(block).style(style(BG).bg(BG));
    f.render_widget(list, area);
}

// ── Status bar ───────────────────────────────────────────────────────────────

fn render_statusbar(f: &mut Frame, app: &App, area: Rect) {
    let (text, col) = if let Some((msg, ok, ts)) = &app.status {
        if ts.elapsed() < Duration::from_secs(5) {
            let c = if *ok { OK } else { DANGER };
            (format!("  ● {msg}"), c)
        } else {
            hint()
        }
    } else {
        hint()
    };

    let bar = Paragraph::new(Span::styled(text, style(col)))
        .style(Style::default().bg(Color::Rgb(18, 18, 36)));
    f.render_widget(bar, area);
}

fn hint() -> (String, Color) {
    (
        "  [Tab] switch tab   [s] sort   [r] reload config   [q] quit".to_string(),
        DIM,
    )
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn style(c: Color) -> Style {
    Style::default().fg(c)
}

fn hdr() -> Style {
    Style::default().fg(WARN).add_modifier(Modifier::BOLD)
}

fn field_block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_style(style(BORDER))
        .title(Span::styled(format!(" {title} "), style(DIM)))
}

fn dim(s: &str) -> Span<'_> {
    Span::styled(s, style(DIM))
}

fn load_span(v: f64) -> Span<'static> {
    let col = if v > 4.0 { DANGER } else if v > 2.0 { WARN } else { OK };
    Span::styled(format!("{v:.2}"), style(col).add_modifier(Modifier::BOLD))
}

fn threshold_color(val: f32, warn: f32, danger: f32) -> Color {
    if val >= danger { DANGER } else if val >= warn { WARN } else { OK }
}

fn fmt_bytes(b: u64) -> String {
    let mb = b / 1024 / 1024;
    if mb >= 1024 { format!("{:.1}G", mb as f64 / 1024.0) } else { format!("{mb}M") }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() > max {
        format!("{}…", s.chars().take(max - 1).collect::<String>())
    } else {
        s.to_string()
    }
}
