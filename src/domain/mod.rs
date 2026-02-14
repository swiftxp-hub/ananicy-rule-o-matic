pub mod models;

use anyhow::Result;
use models::EnrichedRule;

pub trait RuleRepository
{
    fn load_all(&self) -> Result<Vec<EnrichedRule>>;
}
