//! Manage ownership of DNS records
//!
//! Registries are responsible for ensuring that no changes are made to DNS records that we did no create ourselves.
//! Note that this application will only ever modify A records (and TXT records if using the [`TxtRegistry`]), never AAAA records.
//!
//! All registries implement the [`ARegistry`] trait. Currently, the following registries are available:
//! - [`TxtRegistry`]: Manages ownership via TXT records in the same zone as the A records

mod txt;

pub use txt::TxtRegistry;

use std::{
    fmt::Display,
    net::{Ipv4Addr, Ipv6Addr},
};

#[cfg(test)]
use mockall::automock;

/// Tracks the ownership of A records for [`Domain`]s.
/// A record changes may only be made to domains that are owned by a registry.
///
/// This is enforced by only allowing record changes in [`crate::provider::Provider`] through [`crate::plan::Plan`]s,
/// which in turn uses a registry to change domain ownership.
#[cfg_attr(test, automock)]
#[cfg_attr(not(test), allow(clippy::needless_lifetimes))] // needed for mockall
pub trait ARegistry {
    /// Set the registry tenant name
    fn set_tenant(&mut self, tenant: String);
    /// Returns domains currently owned by this registry
    fn owned_domains(&self) -> Vec<Domain>;
    //// Returns all domains that the registry knows about, regardless of ownership status
    fn all_domains(&self) -> Vec<Domain>;
    /// Attempts to claim a domain by name with the registry's backend.
    /// Returns a result containing [`Ok`] if the domain is claimed or a [`RegistryError`] if the domain could not be claimed.
    fn claim<'a>(&mut self, name: DomainName<'a>) -> Result<(), RegistryError>;
    /// Attempt to release a claimed domain with the registry's backend.
    /// Returns a result containing [`Ok`] if the domain is released or a [`RegistryError`] if the domain could not be released.
    fn release<'a>(&mut self, name: DomainName<'a>) -> Result<(), RegistryError>;
}

/// Represents a single FQDN and its associated DNS records, as returned by a [`ARegistry`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Domain {
    pub name: String,
    pub a: Vec<Ipv4Addr>,
    pub aaaa: Vec<Ipv6Addr>,
    pub txt: Vec<String>,
    // Need to ble able to create domains with ownership in tests
    #[cfg(test)]
    pub a_ownership: Ownership,
    #[cfg(not(test))]
    a_ownership: Ownership,
}

/// Represents the current ownership status of a domain.
#[derive(Debug, Clone, PartialEq, Eq)]
#[doc(hidden)]
pub enum Ownership {
    /// This domains A record belongs to us
    Owned,
    /// This domains A records are managed by someone else
    Taken,
    /// This domain doesn't have A records, we can claim it
    Available,
}

pub type DomainName<'a> = &'a str;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RegistryError {
    msg: String,
}

impl Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.msg.as_str())
    }
}

impl std::error::Error for RegistryError {}

impl From<String> for RegistryError {
    fn from(s: String) -> Self {
        RegistryError { msg: s }
    }
}
