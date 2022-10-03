use std::{
    fmt::Display,
    net::{Ipv4Addr, Ipv6Addr},
};

use crate::{plan::Plan, provider::Provider};

/// Ipv4Registry implements ownership of IPv4 records.
/// Any changes made through a plan are first referenced by the registry
/// to prevent overwriting records not owned by us.
pub trait Ipv4Registry {
    /// Returns a list of domains currently owned by us
    fn owned_domains() -> Vec<Domain>;
    /// Attempt to register a new domain for us. Fails if the domain is already owned by someone else
    fn register_domain(domain: Domain) -> Result<(), RegistryError>;
    /// Apply the given plan to the specified provider, ensuring that ownership is preserved
    fn apply_plan(plan: Plan, provider: dyn Provider);
}

#[derive(Debug)]
pub struct Domain {
    pub name: String,
    pub a: Vec<Ipv4Addr>,
    pub aaaa: Vec<Ipv6Addr>,
    pub txt: Vec<String>,
}

#[derive(Debug)]
pub struct RegistryError {
    msg: String,
}

impl Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg.as_str())
    }
}

impl std::error::Error for RegistryError {}

impl From<String> for RegistryError {
    fn from(s: String) -> Self {
        RegistryError { msg: s }
    }
}
