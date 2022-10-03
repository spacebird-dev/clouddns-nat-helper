mod cloudflare;

use std::fmt::Display;

use crate::config::Config;
use crate::plan::Plan;
use crate::types::DnsRecord;

// Generic error returned by a provider action
#[derive(Debug)]
pub struct ProviderError {
    msg: String,
}

impl Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg.as_str())
    }
}

impl std::error::Error for ProviderError {}

impl From<String> for ProviderError {
    fn from(s: String) -> Self {
        ProviderError { msg: s }
    }
}

// Providers implement a few basic methods to access their cloud DNS registry for reading and writing records
pub trait Provider {
    fn apply_plan(&self, plan: Plan) -> Vec<Result<(), ProviderError>>;
    fn read_records(&self) -> Result<Vec<DnsRecord>, ProviderError>;
}
