mod txt;

pub use txt::TxtRegistry;

use std::{
    fmt::Display,
    net::{Ipv4Addr, Ipv6Addr},
};

/// ARegistry implements ownership of A records.
/// Any changes made through a plan are first referenced by the registry
/// to prevent overwriting records not owned by us.
pub trait ARegistry {
    fn set_tenant(&mut self, tenant: String);
    /// Returns a list of domains currently owned by us
    fn owned_domains(&self) -> Vec<Domain>;
    /// Attempt to claim a domain wit the registrys backend
    fn claim(&mut self, name: &DomainName) -> Result<(), RegistryError>;
    /// Attempt to release a claimed domain
    fn release(&mut self, name: &DomainName) -> Result<(), RegistryError>;
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
