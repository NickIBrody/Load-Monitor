use thiserror::Error;

#[derive(Error, Debug)]
pub enum ResguardError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Configuration error: {0}")]
    Config(String),
    
    #[error("Cgroup error: {0}")]
    Cgroup(String),
    
    #[error("Process error: {0}")]
    Process(String),
    
    #[error("Metric collection failed: {0}")]
    Metrics(String),
    
    #[error("Rule evaluation failed: {0}")]
    Rule(String),
}
