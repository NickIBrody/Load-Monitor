use crate::process::Process;
use crate::rules::Action;
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use std::fs;
use std::path::{Path, PathBuf};

pub struct CgroupLimiter {
    base_path: String,
    created: Vec<PathBuf>,
}

impl CgroupLimiter {
    pub fn new(base_path: &str) -> anyhow::Result<Self> {
        Ok(Self {
            base_path: base_path.to_string(),
            created: Vec::new(),
        })
    }

    pub async fn apply(&mut self, process: &Process, action: Action) -> anyhow::Result<()> {
        match action {
            Action::LimitCpu(max_pct)   => self.limit_cpu(process, max_pct)?,
            Action::LimitMemory(max_b)  => self.limit_memory(process, max_b)?,
            Action::Stop                => self.stop_process(process)?,
        }
        Ok(())
    }

    fn cgroup_path(&self, pid: u32) -> PathBuf {
        Path::new(&self.base_path).join(format!("resguard-{pid}"))
    }

    fn ensure_cgroup(&mut self, pid: u32) -> anyhow::Result<PathBuf> {
        let path = self.cgroup_path(pid);
        if !path.exists() {
            fs::create_dir_all(&path)?;
            self.created.push(path.clone());
        }
        Ok(path)
    }

    fn limit_cpu(&mut self, process: &Process, max_percent: f32) -> anyhow::Result<()> {
        let path = self.ensure_cgroup(process.pid)?;
        let quota = (max_percent * 1_000.0) as u64;
        fs::write(path.join("cpu.max"), format!("{quota} 100000"))?;
        fs::write(path.join("cgroup.procs"), process.pid.to_string())?;
        Ok(())
    }

    fn limit_memory(&mut self, process: &Process, max_bytes: u64) -> anyhow::Result<()> {
        let path = self.ensure_cgroup(process.pid)?;
        fs::write(path.join("memory.max"), max_bytes.to_string())?;
        fs::write(path.join("cgroup.procs"), process.pid.to_string())?;
        Ok(())
    }

    fn stop_process(&self, process: &Process) -> anyhow::Result<()> {
        signal::kill(Pid::from_raw(process.pid as i32), Signal::SIGTERM)?;
        Ok(())
    }

    pub async fn cleanup(&self) {
        // Remove cgroup dirs created by this session
        for path in &self.created {
            let _ = fs::remove_dir(path);
        }
        // Also clean up any leftover resguard-* dirs (e.g. from a crashed previous run)
        if let Ok(entries) = fs::read_dir(&self.base_path) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                if name.to_string_lossy().starts_with("resguard-") {
                    let _ = fs::remove_dir(entry.path());
                }
            }
        }
    }
}
