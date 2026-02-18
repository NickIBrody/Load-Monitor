use crate::config::{Rule as ConfigRule, Condition, Action as ConfigAction};
use crate::process::Process;
use crate::metrics::SystemMetrics;
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct Rule {
    pub name: String,
    pub condition: RuleCondition,
    pub action: Action,
    pub duration: Duration,
}

#[derive(Debug, Clone)]
pub enum RuleCondition {
    CpuOver(f32),
    MemoryOver(u64),
    And(Vec<RuleCondition>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    LimitCpu(f32),
    LimitMemory(u64),
    Stop,
}

pub struct Engine {
    rules: Vec<Rule>,
    violation_tracker: HashMap<String, ViolationState>,
}

struct ViolationState {
    start_time: Instant,
    active: bool,
}

impl Engine {
    pub fn new(config_rules: &[ConfigRule]) -> Self {
        let mut rules = Vec::new();
        
        for rule in config_rules {
            if let Some(r) = Self::parse_rule(rule) {
                rules.push(r);
            }
        }
        
        Self {
            rules,
            violation_tracker: HashMap::new(),
        }
    }
    
    fn parse_rule(config: &ConfigRule) -> Option<Rule> {
        let condition = Self::parse_condition(&config.condition)?;
        let action = Self::parse_action(&config.action)?;
        
        Some(Rule {
            name: config.name.clone(),
            condition,
            action,
            duration: Duration::from_secs(config.duration_secs),
        })
    }
    
    fn parse_condition(cond: &Condition) -> Option<RuleCondition> {
        match cond {
            Condition::CpuOver { threshold } => {
                Some(RuleCondition::CpuOver(*threshold))
            }
            Condition::MemoryOver { threshold } => {
                Some(RuleCondition::MemoryOver(*threshold))
            }
            Condition::And { conditions } => {
                let mut subconditions = Vec::new();
                for c in conditions {
                    if let Some(sc) = Self::parse_condition(c) {
                        subconditions.push(sc);
                    }
                }
                Some(RuleCondition::And(subconditions))
            }
        }
    }
    
    fn parse_action(action: &ConfigAction) -> Option<Action> {
        match action {
            ConfigAction::LimitCpu { max_percent } => Some(Action::LimitCpu(*max_percent)),
            ConfigAction::LimitMemory { max_bytes } => Some(Action::LimitMemory(*max_bytes)),
            ConfigAction::Stop => Some(Action::Stop),
        }
    }
    
    pub fn evaluate(&mut self, process: &Process, _metrics: &SystemMetrics) -> Option<Action> {
        for rule in &self.rules {
            if self.check_condition(&rule.condition, process) {
                let key = format!("{}_{}", rule.name, process.pid);
                
                let state = self.violation_tracker.entry(key).or_insert(ViolationState {
                    start_time: Instant::now(),
                    active: false,
                });
                
                if !state.active && state.start_time.elapsed() >= rule.duration {
                    state.active = true;
                    return Some(rule.action.clone());
                }
            } else {
                self.violation_tracker.remove(&format!("{}_{}", rule.name, process.pid));
            }
        }
        None
    }
    
    fn check_condition(&self, condition: &RuleCondition, process: &Process) -> bool {
        match condition {
            RuleCondition::CpuOver(threshold) => {
                process.cpu_usage > *threshold
            }
            RuleCondition::MemoryOver(threshold) => {
                process.memory > *threshold
            }
            RuleCondition::And(conditions) => {
                conditions.iter().all(|c| self.check_condition(c, process))
            }
        }
    }
}
