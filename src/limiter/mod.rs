use crate::process::Process;
use crate::rules::Action;
use nix::unistd::Pid;
use nix::sys::signal::{self, Signal};
use std::fs;
use std::path::Path;

pub struct CgroupLimiter {
    base_path: String,
}

impl CgroupLimiter {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            base_path: "/sys/fs/cgroup".to_string(),
        })
    }
    
    pub async fn apply(&self, process: &Process, action: Action) -> anyhow::Result<()> {
        match action {
            Action::LimitCpu(max_percent) => {
                self.limit_cpu(process, max_percent).await?;
            }
            Action::LimitMemory(max_bytes) => {
                self.limit_memory(process, max_bytes).await?;
            }
            Action::Stop => {
                self.stop_process(process).await?;
            }
        }
        Ok(())
    }
    
    async fn limit_cpu(&self, process: &Process, max_percent: f32) -> anyhow::Result<()> {
        let cgroup_name = format!("resguard-{}", process.pid);
        let cgroup_path = Path::new(&self.base_path).join(cgroup_name);
        
        fs::create_dir_all(&cgroup_path)?;
        
        let quota = (max_percent * 1000.0) as i64;
        fs::write(cgroup_path.join("cpu.max"), format!("{} 100000", quota))?;
        
        fs::write(cgroup_path.join("cgroup.procs"), process.pid.to_string())?;
        
        Ok(())
    }
    
    async fn limit_memory(&self, process: &Process, max_bytes: u64) -> anyhow::Result<()> {
        let cgroup_name = format!("resguard-{}", process.pid);
        let cgroup_path = Path::new(&self.base_path).join(cgroup_name);
        
        fs::create_dir_all(&cgroup_path)?;
        
        fs::write(cgroup_path.join("memory.max"), max_bytes.to_string())?;
        fs::write(cgroup_path.join("cgroup.procs"), process.pid.to_string())?;
        
        Ok(())
    }
    
    async fn stop_process(&self, process: &Process) -> anyhow::Result<()> {
        let pid = Pid::from_raw(process.pid as i32);
        signal::kill(pid, Signal::SIGTERM)?;
        Ok(())
    }
}
