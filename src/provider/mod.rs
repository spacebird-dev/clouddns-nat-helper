use domain::rdata;
use std::fmt::Display;

// Provider submodules
mod cloudflare;

type Zone = String;

// Generic error returned by a provider action
#[derive(Debug, Clone)]
pub struct ProviderError {
    message: String,
}

impl From<&String> for ProviderError {
    fn from(s: &String) -> Self {
        ProviderError {
            message: s.to_string(),
        }
    }
}
impl std::error::Error for ProviderError {}
impl Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Provider Error: {}", self.message)
    }
}

// A Domain is a single domain name and may have associated records
#[derive(Debug, Clone)]
pub struct Domain<'a> {
    pub zone: &'a Zone,
    pub a_records: Vec<rdata::Aaaa>,
    pub aaaa_records: Vec<rdata::A>,
    pub txt_records: Vec<rdata::Txt<String>>,
}

// Providers implement a few basic methods to access their cloud DNS registry
pub trait Provider {
    fn create(config: &super::config::Config) -> Result<Box<dyn Provider>, ProviderError>
    where
        Self: Sized;
    fn all_zones(&self) -> Result<Vec<Zone>, ProviderError>;
    fn zone_domains(&self, zone: &Zone) -> Result<Vec<Domain>, ProviderError>;
    fn update_domain_a_records(&self, domain: &Domain) -> Result<(), ProviderError>;
}
