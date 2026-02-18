use sysinfo::{System, SystemExt, CpuExt};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub cpu_total: f32,
    pub memory_used: u64,
    pub memory_total: u64,
    pub load_average: LoadAvg,
    pub timestamp: std::time::SystemTime,
}

#[derive(Debug, Clone)]
pub struct LoadAvg {
    pub one: f64,
    pub five: f64,
    pub fifteen: f64,
}

pub struct Collector {
    system: System,
}

impl Collector {
    pub fn new() -> Self {
        Self {
            system: System::new_all(),
        }
    }
    
    pub async fn collect(&mut self) -> anyhow::Result<SystemMetrics> {
        self.system.refresh_all();
        tokio::task::spawn_blocking(move || {
            std::thread::sleep(Duration::from_millis(100));
        }).await?;
        
        let load_avg = self.system.load_average();
        
        Ok(SystemMetrics {
            cpu_total: self.system.global_cpu_info().cpu_usage(),
            memory_used: self.system.used_memory(),
            memory_total: self.system.total_memory(),
            load_average: LoadAvg {
                one: load_avg.one,
                five: load_avg.five,
                fifteen: load_avg.fifteen,
            },
            timestamp: std::time::SystemTime::now(),
        })
    }
}
