mod config;
mod metrics;
mod process;
mod rules;
mod limiter;
mod errors;

use tokio::time;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug)]
struct ActionLog {
    timestamp: chrono::DateTime<chrono::Utc>,
    process: process::Process,
    action: rules::Action,
}

struct AppState {
    actions: Vec<ActionLog>,
}

impl AppState {
    fn new() -> Self {
        Self { actions: Vec::new() }
    }
    
    fn log_action(&mut self, process: process::Process, action: rules::Action) {
        self.actions.push(ActionLog {
            timestamp: chrono::Utc::now(),
            process,
            action,
        });
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    
    let config = config::Config::load("config.toml")?;
    println!("✅ Resguard started. Monitoring processes...");
    println!("📊 Check interval: {} sec", config.general.interval_secs);
    println!("⚙️  Rules loaded: {}", config.rules.len());
    
    let state = Arc::new(RwLock::new(AppState::new()));
    
    let mut metrics_collector = metrics::Collector::new();
    let mut rule_engine = rules::Engine::new(&config.rules);
    let limiter = limiter::CgroupLimiter::new()?;
    
    let mut interval = time::interval(time::Duration::from_secs(config.general.interval_secs));
    
    loop {
        interval.tick().await;
        
        let metrics = metrics_collector.collect().await?;
        let processes = process::Scanner::scan().await?;
        
        println!("\n--- {} ---", chrono::Local::now().format("%H:%M:%S"));
        println!("Total CPU: {:.1}%", metrics.cpu_total);
        println!("Memory: {} MB / {} MB", 
            metrics.memory_used / 1024 / 1024,
            metrics.memory_total / 1024 / 1024
        );
        
        let mut top_processes: Vec<&process::Process> = processes.iter()
            .filter(|p| p.cpu_usage > 0.1)
            .collect();
        
        top_processes.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap());
        
        println!("\n🔥 Top CPU processes:");
        if top_processes.is_empty() {
            println!("  No active processes (>0.1% CPU)");
        } else {
            for proc in top_processes.iter().take(5) {
                println!("  PID {:6}: {:<20} - CPU: {:5.1}%   RAM: {} MB", 
                    proc.pid, 
                    if proc.name.len() > 20 { &proc.name[..20] } else { &proc.name },
                    proc.cpu_usage,
                    proc.memory / 1024 / 1024
                );
            }
        }
        
        for proc in processes {
            if let Some(action) = rule_engine.evaluate(&proc, &metrics) {
                println!("⚠️  APPLYING {:?} to PID {} ({}) - CPU: {:.1}%", 
                    action, proc.pid, proc.name, proc.cpu_usage
                );
                log::info!("Applying {:?} to PID {}", action, proc.pid);
                
                match limiter.apply(&proc, action.clone()).await {
                    Ok(_) => println!("✅ Limit applied"),
                    Err(e) => println!("❌ Error: {}", e),
                }
                
                let mut state = state.write().await;
                state.log_action(proc, action);
            }
        }
    }
}
