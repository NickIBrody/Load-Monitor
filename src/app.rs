use crate::config::Config;
use crate::limiter::CgroupLimiter;
use crate::metrics::{Collector, SystemMetrics};
use crate::process::Process;
use crate::rules::{Action, Engine};
use crate::ui;
use anyhow::Result;
use chrono::{DateTime, Local};
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::{Duration, Instant};

// ── Tab ──────────────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq)]
pub enum Tab {
    Monitor,
    Rules,
    Settings,
    Log,
}

impl Tab {
    pub fn index(&self) -> usize {
        match self {
            Tab::Monitor  => 0,
            Tab::Rules    => 1,
            Tab::Settings => 2,
            Tab::Log      => 3,
        }
    }

    fn from_index(i: usize) -> Self {
        match i % 4 {
            0 => Tab::Monitor,
            1 => Tab::Rules,
            2 => Tab::Settings,
            3 => Tab::Log,
            _ => Tab::Monitor,
        }
    }
}

// ── Sort ─────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub enum SortBy {
    Cpu,
    Memory,
    Pid,
    Name,
}

impl SortBy {
    pub fn label(&self) -> &'static str {
        match self {
            SortBy::Cpu    => "CPU ↓",
            SortBy::Memory => "RAM ↓",
            SortBy::Pid    => "PID ↑",
            SortBy::Name   => "Name ↑",
        }
    }

    fn next(&self) -> Self {
        match self {
            SortBy::Cpu    => SortBy::Memory,
            SortBy::Memory => SortBy::Pid,
            SortBy::Pid    => SortBy::Name,
            SortBy::Name   => SortBy::Cpu,
        }
    }
}

// ── ActionLog ────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct ActionLog {
    pub timestamp: DateTime<Local>,
    pub pid: u32,
    pub name: String,
    pub action: String,
    pub success: bool,
}

// ── App ──────────────────────────────────────────────────────────────────────

pub struct App {
    pub config: Config,
    pub config_path: String,

    pub current_tab: Tab,
    pub metrics: Option<SystemMetrics>,
    pub processes: Vec<Process>,
    pub action_log: Vec<ActionLog>,

    pub selected_process: usize,
    pub sort_by: SortBy,

    pub log_scroll: usize,

    pub settings_selected: usize,
    pub settings_editing: bool,
    pub settings_edit_buf: String,

    pub status: Option<(String, bool, Instant)>,
    pub running: bool,
}

impl App {
    pub fn new(config_path: &str) -> Result<Self> {
        let config = Config::load(config_path)?;
        Ok(Self {
            config,
            config_path: config_path.to_string(),
            current_tab: Tab::Monitor,
            metrics: None,
            processes: Vec::new(),
            action_log: Vec::new(),
            selected_process: 0,
            sort_by: SortBy::Cpu,
            log_scroll: 0,
            settings_selected: 0,
            settings_editing: false,
            settings_edit_buf: String::new(),
            status: None,
            running: true,
        })
    }

    pub async fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        let mut collector   = Collector::new();
        let mut rule_engine = Engine::new(&self.config.rules);
        let mut limiter     = CgroupLimiter::new(&self.config.limits.cgroup_base_path)?;

        let mut last_tick = Instant::now() - Duration::from_secs(999); // force immediate first tick

        while self.running {
            terminal.draw(|f| ui::render(f, self))?;

            // Poll input with short timeout so UI stays responsive
            if event::poll(Duration::from_millis(80))? {
                if let Event::Key(key) = event::read()? {
                    self.handle_key(key, &mut rule_engine, &mut limiter);
                }
            }

            let tick_interval = Duration::from_secs(self.config.general.interval_secs);
            if last_tick.elapsed() >= tick_interval {
                last_tick = Instant::now();
                self.tick(&mut collector, &mut rule_engine, &mut limiter).await;
            }
        }

        limiter.cleanup().await;
        Ok(())
    }

    async fn tick(
        &mut self,
        collector: &mut Collector,
        rule_engine: &mut Engine,
        limiter: &mut CgroupLimiter,
    ) {
        match collector.collect().await {
            Ok(m) => self.metrics = Some(m),
            Err(e) => self.set_status(format!("Metrics error: {e}"), false),
        }

        match crate::process::Scanner::scan().await {
            Ok(mut procs) => {
                self.sort_vec(&mut procs);
                self.processes = procs;
            }
            Err(e) => self.set_status(format!("Scan error: {e}"), false),
        }

        if let Some(metrics) = self.metrics.clone() {
            let procs = self.processes.clone();
            for proc in &procs {
                if self.config.limits.blacklist.iter().any(|b| proc.name.contains(b.as_str())) {
                    continue;
                }
                if let Some(action) = rule_engine.evaluate(proc, &metrics) {
                    let action_str = action_label(&action);
                    let success = limiter.apply(proc, action).await.is_ok();
                    self.push_log(proc.pid, proc.name.clone(), action_str, success);
                }
            }
        }
    }

    fn push_log(&mut self, pid: u32, name: String, action: String, success: bool) {
        self.action_log.push(ActionLog {
            timestamp: Local::now(),
            pid,
            name,
            action,
            success,
        });
        let max = self.config.general.history_size;
        if self.action_log.len() > max {
            self.action_log.drain(..self.action_log.len() - max);
        }
    }

    fn sort_vec(&self, procs: &mut Vec<Process>) {
        match self.sort_by {
            SortBy::Cpu    => procs.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap_or(std::cmp::Ordering::Equal)),
            SortBy::Memory => procs.sort_by(|a, b| b.memory.cmp(&a.memory)),
            SortBy::Pid    => procs.sort_by_key(|p| p.pid),
            SortBy::Name   => procs.sort_by(|a, b| a.name.cmp(&b.name)),
        }
    }

    pub fn set_status(&mut self, msg: String, ok: bool) {
        self.status = Some((msg, ok, Instant::now()));
    }

    // ── Key handling ─────────────────────────────────────────────────────────

    fn handle_key(
        &mut self,
        key: crossterm::event::KeyEvent,
        rule_engine: &mut Engine,
        _limiter: &mut CgroupLimiter,
    ) {
        // Global quit
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.running = false;
            return;
        }
        if key.code == KeyCode::Char('q') && !self.settings_editing {
            self.running = false;
            return;
        }

        // Tab switching (global, not when editing)
        if !self.settings_editing {
            match key.code {
                KeyCode::Tab => {
                    self.current_tab = Tab::from_index(self.current_tab.index() + 1);
                    return;
                }
                KeyCode::BackTab => {
                    self.current_tab = Tab::from_index(self.current_tab.index() + 3);
                    return;
                }
                KeyCode::Char('1') => { self.current_tab = Tab::Monitor;  return; }
                KeyCode::Char('2') => { self.current_tab = Tab::Rules;    return; }
                KeyCode::Char('3') => { self.current_tab = Tab::Settings; return; }
                KeyCode::Char('4') => { self.current_tab = Tab::Log;      return; }
                _ => {}
            }
        }

        match self.current_tab.clone() {
            Tab::Monitor  => self.key_monitor(key, rule_engine),
            Tab::Settings => self.key_settings(key),
            Tab::Log      => self.key_log(key),
            Tab::Rules    => {}
        }
    }

    fn key_monitor(&mut self, key: crossterm::event::KeyEvent, _rule_engine: &mut Engine) {
        let len = self.processes.len();
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected_process + 1 < len {
                    self.selected_process += 1;
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_process > 0 {
                    self.selected_process -= 1;
                }
            }
            KeyCode::Char('s') => {
                self.sort_by = self.sort_by.next();
                let mut procs = self.processes.clone();
                self.sort_vec(&mut procs);
                self.processes = procs;
                self.selected_process = 0;
            }
            KeyCode::Char('r') => {
                match Config::load(&self.config_path) {
                    Ok(cfg) => {
                        self.config = cfg;
                        self.set_status("Config reloaded".to_string(), true);
                    }
                    Err(e) => self.set_status(format!("Reload failed: {e}"), false),
                }
            }
            _ => {}
        }
    }

    fn key_settings(&mut self, key: crossterm::event::KeyEvent) {
        const FIELD_COUNT: usize = 4;

        if self.settings_editing {
            match key.code {
                KeyCode::Esc => {
                    self.settings_editing = false;
                    self.settings_edit_buf.clear();
                }
                KeyCode::Enter => {
                    self.apply_settings_edit();
                    self.settings_editing = false;
                    self.settings_edit_buf.clear();
                }
                KeyCode::Backspace => {
                    self.settings_edit_buf.pop();
                }
                KeyCode::Char(c) => {
                    self.settings_edit_buf.push(c);
                }
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.settings_selected + 1 < FIELD_COUNT {
                        self.settings_selected += 1;
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.settings_selected > 0 {
                        self.settings_selected -= 1;
                    }
                }
                KeyCode::Enter | KeyCode::Char('e') => {
                    self.settings_edit_buf = self.current_field_value();
                    self.settings_editing = true;
                }
                KeyCode::Char('w') => match self.config.save(&self.config_path) {
                    Ok(_)  => self.set_status("Saved to config.toml".to_string(), true),
                    Err(e) => self.set_status(format!("Save failed: {e}"), false),
                },
                _ => {}
            }
        }
    }

    fn key_log(&mut self, key: crossterm::event::KeyEvent) {
        let len = self.action_log.len();
        match key.code {
            KeyCode::Down | KeyCode::Char('j') => {
                if self.log_scroll + 1 < len {
                    self.log_scroll += 1;
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.log_scroll > 0 {
                    self.log_scroll -= 1;
                }
            }
            KeyCode::Char('c') => {
                self.action_log.clear();
                self.log_scroll = 0;
                self.set_status("Log cleared".to_string(), true);
            }
            _ => {}
        }
    }

    // ── Settings field helpers ───────────────────────────────────────────────

    fn current_field_value(&self) -> String {
        match self.settings_selected {
            0 => self.config.general.interval_secs.to_string(),
            1 => self.config.general.history_size.to_string(),
            2 => format!("{:.1}", self.config.limits.default_cpu_quota),
            3 => self.config.limits.default_memory_limit.to_string(),
            _ => String::new(),
        }
    }

    fn apply_settings_edit(&mut self) {
        let v = self.settings_edit_buf.trim().to_string();
        let ok = match self.settings_selected {
            0 => v.parse::<u64>().ok().filter(|&n| n > 0).map(|n| self.config.general.interval_secs = n).is_some(),
            1 => v.parse::<usize>().ok().map(|n| self.config.general.history_size = n).is_some(),
            2 => v.parse::<f32>().ok().filter(|&n| n > 0.0 && n <= 100.0).map(|n| self.config.limits.default_cpu_quota = n).is_some(),
            3 => v.parse::<u64>().ok().map(|n| self.config.limits.default_memory_limit = n).is_some(),
            _ => false,
        };
        if ok {
            self.set_status("Applied — press [w] to save".to_string(), true);
        } else {
            self.set_status("Invalid value".to_string(), false);
        }
    }
}

fn action_label(a: &Action) -> String {
    match a {
        Action::LimitCpu(p)    => format!("LimitCPU → {p:.0}%"),
        Action::LimitMemory(b) => format!("LimitRAM → {}MB", b / 1024 / 1024),
        Action::Stop           => "SIGTERM".to_string(),
    }
}
