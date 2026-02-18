use sysinfo::{System, SystemExt, ProcessExt, PidExt};
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
    pub fn new() -> Self {
        Self
    }
    
    pub async fn scan() -> anyhow::Result<Vec<Process>> {
        let mut system = System::new_all();
        system.refresh_all();
        system.refresh_processes();
        
        let mut processes = Vec::new();
        
        for (pid, proc) in system.processes() {
            let exe_path = proc.exe();
            let exe_string = exe_path.to_string_lossy().to_string();
            
            let process = Process {
                pid: pid.as_u32(),
                name: proc.name().to_string(),
                exe: exe_string,
                cmd: proc.cmd().to_vec(),
                cpu_usage: proc.cpu_usage(),
                memory: proc.memory(),
                user: "unknown".to_string(),
                systemd_service: Self::detect_systemd_service(pid.as_u32()),
            };
            
            processes.push(process);
        }
        
        Ok(processes)
    }
    
    fn detect_systemd_service(pid: u32) -> Option<String> {
        let path = format!("/proc/{}/cgroup", pid);
        if let Ok(content) = std::fs::read_to_string(path) {
            for line in content.lines() {
                if line.contains("system.slice") {
                    if let Some(service) = line.split('/').last() {
                        return Some(service.to_string());
                    }
                }
            }
        }
        None
    }
}

pub struct Matcher {
    whitelist: Vec<Regex>,
    blacklist: Vec<Regex>,
}

impl Matcher {
    pub fn new(whitelist: Vec<String>, blacklist: Vec<String>) -> anyhow::Result<Self> {
        Ok(Self {
            whitelist: Self::compile_patterns(whitelist)?,
            blacklist: Self::compile_patterns(blacklist)?,
        })
    }
    
    fn compile_patterns(patterns: Vec<String>) -> anyhow::Result<Vec<Regex>> {
        patterns
            .into_iter()
            .map(|p| Regex::new(&p).map_err(|e| anyhow::anyhow!("Invalid regex: {}", e)))
            .collect()
    }
    
    pub fn should_monitor(&self, process: &Process) -> bool {
        if !self.blacklist.is_empty() {
            for pattern in &self.blacklist {
                if pattern.is_match(&process.name) || pattern.is_match(&process.exe) {
                    return false;
                }
            }
        }
        
        if !self.whitelist.is_empty() {
            for pattern in &self.whitelist {
                if pattern.is_match(&process.name) || pattern.is_match(&process.exe) {
                    return true;
                }
            }
            return false;
        }
        
        true
    }
}
