mod cloudflare;

use crate::config::Config;
use crate::plan::Plan;
use crate::types::DnsRecord;

// Generic error returned by a provider action
pub type ProviderError = String;

// Providers implement a few basic methods to access their cloud DNS registry
pub trait Provider {
    fn from_config(config: &Config) -> Result<Box<dyn Provider>, ProviderError>
    where
        Self: Sized;
    fn apply_plan(&self, plan: Plan) -> Vec<Result<(), ProviderError>>;
    fn read_records(&self) -> Result<Vec<DnsRecord>, ProviderError>;
}
