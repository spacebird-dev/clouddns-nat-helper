mod cloudflare;

// Re-exports for convenience
pub use self::cloudflare::{CloudflareProvider, CloudflareProviderConfig};

// Actual provider uses
use std::{
    fmt::Display,
    net::{Ipv4Addr, Ipv6Addr},
};

use crate::{config::TTL, plan::Plan};

// Providers implement a few basic methods to access their cloud DNS registry for reading and writing records
pub trait Provider: Sync {
    /// Returns whether this provider supports running in dry-run mode, with no changes being made
    fn supports_dry_run(&self) -> bool;
    fn set_dry_run(&mut self, dry_run: bool);

    fn ttl(&self) -> Option<TTL>;
    fn set_ttl(&mut self, ttl: TTL);

    /// Get all records currently registered with the provider
    fn records(&self) -> Result<Vec<DnsRecord>, ProviderError>;

    /// Apply a full plan of DNS record changes to this provider
    fn apply_plan(&self, plan: Plan) -> Vec<Result<(), ProviderError>>;
    /// Create a single DNS record
    fn create_record(&self, record: DnsRecord) -> Result<(), ProviderError>;
    /// Delete a single DNS record
    fn delete_record(&self, record: DnsRecord) -> Result<(), ProviderError>;
}

// Generic error returned by a provider action
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DnsRecord {
    pub domain: String,
    pub content: RecordContent,
    pub ttl: Option<u32>,
}
impl Display for DnsRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.domain, self.content)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RecordContent {
    A(Ipv4Addr),
    Aaaa(Ipv6Addr),
    Txt(String),
}
impl Display for RecordContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                RecordContent::A(a) => format!("A {}", a),
                RecordContent::Aaaa(aaaa) => format!("AAAA {}", aaaa),
                RecordContent::Txt(txt) => format!("TXT {}", txt),
            }
        )
    }
}
