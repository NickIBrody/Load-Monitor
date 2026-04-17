use sysinfo::{ProcessExt, PidExt, System, SystemExt};
use regex::Regex;

#[derive(Debug, Clone)]
pub struct Process {
    pub pid: u32,
    pub name: String,
    pub exe: String,
    pub cmd: Vec<String>,
    pub cpu_usage: f32,
    pub memory: u64,
    pub user: String,
    pub systemd_service: Option<String>,
}

pub struct Scanner;

impl Scanner {
    pub async fn scan() -> anyhow::Result<Vec<Process>> {
        let mut system = System::new_all();
        system.refresh_all();
        system.refresh_processes();

        let processes = system.processes().iter().map(|(pid, proc)| {
            Process {
                pid: pid.as_u32(),
                name: proc.name().to_string(),
                exe: proc.exe().to_string_lossy().to_string(),
                cmd: proc.cmd().to_vec(),
                cpu_usage: proc.cpu_usage(),
                memory: proc.memory(),
                user: "—".to_string(),
                systemd_service: read_systemd_service(pid.as_u32()),
            }
        }).collect();

        Ok(processes)
    }
}

fn read_systemd_service(pid: u32) -> Option<String> {
    let content = std::fs::read_to_string(format!("/proc/{pid}/cgroup")).ok()?;
    for line in content.lines() {
        if line.contains("system.slice") {
            if let Some(svc) = line.split('/').last() {
                if !svc.is_empty() {
                    return Some(svc.to_string());
                }
            }
        }
    }
    None
}

pub struct Matcher {
    whitelist: Vec<Regex>,
    blacklist: Vec<Regex>,
}

impl Matcher {
    pub fn new(whitelist: Vec<String>, blacklist: Vec<String>) -> anyhow::Result<Self> {
        Ok(Self {
            whitelist: compile_patterns(whitelist)?,
            blacklist: compile_patterns(blacklist)?,
        })
    }

    pub fn should_monitor(&self, process: &Process) -> bool {
        for pattern in &self.blacklist {
            if pattern.is_match(&process.name) || pattern.is_match(&process.exe) {
                return false;
            }
        }
        if !self.whitelist.is_empty() {
            return self.whitelist.iter().any(|p| {
                p.is_match(&process.name) || p.is_match(&process.exe)
            });
        }
        true
    }
}

fn compile_patterns(patterns: Vec<String>) -> anyhow::Result<Vec<Regex>> {
    patterns
        .into_iter()
        .map(|p| Regex::new(&p).map_err(|e| anyhow::anyhow!("Invalid regex '{p}': {e}")))
        .collect()
}
