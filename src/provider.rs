//! Interface with DNS providers and get/set zone records.
//!
//! Providers are DNS server providers such as Cloudflare that can be accessed through an API.
//! All providers must implement the [`Provider`] trait. Currently, the following providers are available:
//! - [`CloudflareProvider`]: Interfaces with the Cloudflare dns and zone API
mod cloudflare;

// Re-exports for convenience
pub use self::cloudflare::{CloudflareProvider, CloudflareProviderConfig};

use crate::plan::Action;
#[cfg(test)]
use mockall::{automock, mock};
use std::{
    fmt::Display,
    net::{Ipv4Addr, Ipv6Addr},
};
use thiserror::Error;

/// Trait that provides methods for managing [`DnsRecord`]s using [`Action`]s.
/// Used to interface with DNS providers such as Cloudflare, PowerDNS, etc.
#[cfg_attr(test, automock)]
pub trait DnsProvider {
    /// Tell the provider to not apply any changes, only to pretend doing so. Returns an Error if the provider does not support dry-run mode.
    fn enable_dry_run(&mut self) -> Result<(), ProviderError>;
    /// Whether the provider is currently running in dry-run mode
    fn dry_run(&self) -> bool;

    /// Returns the default ttl that will be applied to all new records
    fn ttl(&self) -> Option<TTL>;
    /// Set a TTL that the provider should apply to all created records
    fn set_ttl(&mut self, ttl: TTL);

    /// Get all relevant records currently registered with the provider.
    /// Note that we only care about A and AAAA records, as well as TXT records (for the [`crate::registry::TxtRegistry`]).
    /// Returns a result of [`DnsRecord`]s
    fn records(&self) -> Result<Vec<DnsRecord>, ProviderError>;

    /// Perform a single Action such as Create, Update or Delete.
    fn apply(&self, action: &Action) -> Result<(), ProviderError>;
}

/// Trait to be implemented by DNS providers that provides methods for managing TXT records.
/// This trait is required by the [`crate::registry::TxtRegistry`] as it manages ownership through TXT records.
#[cfg_attr(test, automock)]
pub trait TxTRegistryProvider {
    /// Create a single TXT record.
    /// This method is intended for use by registries that need to store additional information in the DNS zone,
    /// such as [`crate::registry::TxtRegistry`].
    fn create_txt_record(&self, domain: String, content: String) -> Result<(), ProviderError>;
    /// Delete a single TXT record.
    /// This method is intended for use by registries that need to store additional information in the DNS zone,
    /// such as the [`crate::registry::TxtRegistry`].
    fn delete_txt_record(&self, domain: String, content: String) -> Result<(), ProviderError>;
}

/// A provider represents a DNS service provider such as Cloudflare.
/// They must be able to read and write DNS records, both for updating the actual A records and for managing ownership via TXT records when using the
/// [`crate::registry::TxtRegistry`]
pub trait Provider: DnsProvider + TxTRegistryProvider {}
#[cfg(test)]
mock! {
    pub Provider {}
    impl DnsProvider for Provider {
        fn enable_dry_run(&mut self) -> Result<(), ProviderError>;
        fn dry_run(&self) -> bool;
        fn ttl(&self) -> Option<TTL>;
        fn set_ttl(&mut self, ttl: TTL);
        fn records(&self) -> Result<Vec<DnsRecord>, ProviderError>;
        fn apply(&self, action: &Action) -> Result<(), ProviderError>;
    }
    impl TxTRegistryProvider for Provider {
        fn create_txt_record(&self, domain: String, content: String) -> Result<(), ProviderError>;
        fn delete_txt_record(&self, domain: String, content: String) -> Result<(), ProviderError>;
    }
    impl Provider for Provider {}
}

/// Generic error returned by providers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
pub enum ProviderError {
    #[error("The selected provider does not support dry-run mode")]
    DryRunNotSupported,
    #[error("Internal provider Error: `{0}`")]
    Internal(String),
}
impl From<String> for ProviderError {
    fn from(s: String) -> Self {
        ProviderError::Internal(s)
    }
}

/// Represents a single DNS record as returned by a [`Provider`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DnsRecord {
    /// The fully-qualified domain name of the record (e.g. `my.example.com`)
    pub domain_name: String,
    /// A variant of [`RecordContent`], representing the data stored in the record
    pub content: RecordContent,
}
impl Display for DnsRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.domain_name, self.content)
    }
}
impl PartialEq<&DnsRecord> for DnsRecord {
    fn eq(&self, other: &&DnsRecord) -> bool {
        self == other
    }
}

/// Represents the content of a single [`DnsRecord`].
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

// Desired TTL of managed records
pub type TTL = u32;
