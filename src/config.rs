use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub general: GeneralConfig,
    pub limits: LimitsConfig,
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GeneralConfig {
    pub interval_secs: u64,
    pub history_size: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LimitsConfig {
    pub cgroup_base_path: String,
    pub default_cpu_quota: f32,
    pub default_memory_limit: u64,
    pub blacklist: Vec<String>,
    pub whitelist: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Rule {
    pub name: String,
    pub condition: Condition,
    pub action: Action,
    pub duration_secs: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum Condition {
    CpuOver { threshold: f32 },
    MemoryOver { threshold: u64 },
    And { conditions: Vec<Condition> },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum Action {
    LimitCpu { max_percent: f32 },
    LimitMemory { max_bytes: u64 },
    Stop,
}

impl Config {
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let contents = fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Cannot read {path}: {e}"))?;
        let config: Config = toml::from_str(&contents)
            .map_err(|e| anyhow::anyhow!("Invalid config: {e}"))?;
        Ok(config)
    }

    pub fn save(&self, path: &str) -> anyhow::Result<()> {
        let contents = toml::to_string_pretty(self)
            .map_err(|e| anyhow::anyhow!("Serialize error: {e}"))?;
        fs::write(path, contents)?;
        Ok(())
    }
}
