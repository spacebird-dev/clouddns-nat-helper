mod txt;

pub use txt::TxtRegistry;

use std::{
    fmt::Display,
    net::{Ipv4Addr, Ipv6Addr},
};

#[cfg(test)]
use mockall::automock;

/// ARegistry tracks the ownership of A records for domains.
/// A record changes may only be made to domains that are owned by a registry.
/// This is enforced by only allowing record changes through plans,
/// which in turn need to be created with the help from a registry.
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

/// A domain represents a single namespace of DNS records, including all the A/AAAA/TXT records associated with it.
/// Domains can be owned by either us or someone else, allowing for basic prevention of conflicts.
/// Note that ownership only applies to the domains A records, nat-helper never claims ownership of any other record types
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
