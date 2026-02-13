use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnanicyRuleData
{
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub rule_type: Option<String>,
    pub nice: Option<i32>,
    pub ioclass: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EnrichedRule
{
    pub data: AnanicyRuleData,
    pub context_comment: Option<String>,
    pub source_file: PathBuf,
}
