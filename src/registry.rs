mod txt;

pub use txt::TxtRegistry;

use std::{
    fmt::Display,
    net::{Ipv4Addr, Ipv6Addr},
};

/// ARegistry tracks the ownership of A records for domains.
/// A record changes may only be made to domains that are owned by a registry.
/// This is enforced by only allowing record changes through plans,
/// which in turn need to be created with the help from a registry.
pub trait ARegistry {
    /// Set the registry tenant name
    fn set_tenant(&mut self, tenant: String);
    /// Returns a list of domains currently owned by this registry
    fn owned_domains(&self) -> Vec<Domain>;
    /// Attempts to claim a domain by name with the registry's backend.
    /// Returns a result containing [`Ok`] if the domain is claimed or a [`RegistryError`] if the domain could not be claimed.
    fn claim(&mut self, name: DomainName) -> Result<(), RegistryError>;
    /// Attempt to release a claimed domain with the registry's backend.
    /// Returns a result containing [`Ok`] if the domain is released or a [`RegistryError`] if the domain could not be released.
    fn release(&mut self, name: DomainName) -> Result<(), RegistryError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Domain {
    pub name: DomainName,
    pub a: Vec<Ipv4Addr>,
    pub aaaa: Vec<Ipv6Addr>,
    pub txt: Vec<String>,
}

pub type DomainName = String;

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
