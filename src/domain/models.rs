use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig
{
    pub rule_paths: Vec<PathBuf>,
}

impl Default for AppConfig
{
    fn default() -> Self
    {
        Self {
            rule_paths: vec![
                PathBuf::from("/etc/ananicy.d"),
                PathBuf::from("/usr/lib/ananicy.d"),
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".config/ananicy.d"),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnanicyRule
{
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub rule_type: Option<String>,
    pub nice: Option<i32>,
    pub latency_nice: Option<i32>,
    pub sched: Option<String>,
    pub rtprio: Option<i32>,
    pub ioclass: Option<String>,
    pub oom_score_adj: Option<i32>,
    pub cgroup: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EnrichedRule
{
    pub data: AnanicyRule,
    pub context_comment: Option<String>,
    pub source_file: PathBuf,
    pub shadowed: bool,
}

#[derive(Debug, Clone)]
pub struct ProcessInfo
{
    pub pid: i32,
    pub name: String,
    pub nice: Option<i32>,
    pub oom_score_adj: Option<i32>,
    pub cgroup: Option<String>,
    pub sched_policy: Option<String>,
    pub rtprio: Option<i32>,
    pub ioclass: Option<String>,
    pub latency_nice: Option<i32>,
}
