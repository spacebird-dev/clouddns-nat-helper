//! Manage ownership of DNS records
//!
//! Registries are responsible for ensuring that no changes are made to DNS records that we did no create ourselves.
//!
//! All registries must implement the [`ARegistry`] trait. Currently, the following registries are available:
//! - [`TxtRegistry`]: Manages ownership via TXT records in the same zone as the A records
mod txt;

// Expose individual registry types for creation
pub use txt::TxtRegistry;

use itertools::Itertools;
#[cfg(test)]
use mockall::automock;
use std::net::{Ipv4Addr, Ipv6Addr};
use thiserror::Error;

/// Tracks the ownership of A records for [`Domain`]s.
/// A record changes should only be made to domains that are owned by a registry.
#[cfg_attr(test, automock)]
pub trait ARegistry {
    /// Tell the registry to not apply any changes, only to pretend doing so. Returns an Error if the registry does not support dry-run mode.
    fn enable_dry_run(&mut self) -> Result<(), RegistryError>;

    /// Set the registry tenant name
    fn set_tenant(&mut self, tenant: String);
    //// Returns all domains that the registry knows about
    fn all_domains(&self) -> Vec<Domain>;
    /// Returns domains currently owned by this registry
    fn owned_domains(&self) -> Vec<Domain> {
        self.all_domains()
            .into_iter()
            .filter(|d| matches!(d.ownership(), Ownership::Owned))
            .collect_vec()
    }
    /// Returns domains currently owned by another registry
    fn taken_domains(&self) -> Vec<Domain> {
        self.all_domains()
            .into_iter()
            .filter(|d| matches!(d.ownership(), Ownership::Taken))
            .collect_vec()
    }
    /// Returns domains currently not owned by any registry
    fn available_domains(&self) -> Vec<Domain> {
        self.all_domains()
            .into_iter()
            .filter(|d| matches!(d.ownership(), Ownership::Available))
            .collect_vec()
    }

    /// Attempts to claim a domain by name with the registry's backend.
    /// Returns a result containing [`Ok`] if the domain is claimed or a [`RegistryError`] if the domain could not be claimed.
    #[allow(clippy::needless_lifetimes)] // needed for mockall
    fn claim(&mut self, name: &str) -> Result<(), RegistryError>;
    /// Attempt to release a claimed domain with the registry's backend.
    /// Returns a result containing [`Ok`] if the domain is released or a [`RegistryError`] if the domain could not be released.
    #[allow(clippy::needless_lifetimes)] // needed for mockall
    fn release(&mut self, name: &str) -> Result<(), RegistryError>;
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
impl Domain {
    pub fn ownership(&self) -> Ownership {
        self.a_ownership
    }
}

/// Represents the current ownership status of a domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc(hidden)]
pub enum Ownership {
    /// This domains A record belongs to us
    Owned,
    /// This domains A records are managed by someone else
    Taken,
    /// This domain doesn't have A records and is not taken, we can claim it
    Available,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Error)]
pub enum RegistryError {
    #[error("The selected registry does not support dry-run mode")]
    DryRunNotSupported,
    #[error("Could not claim domain {domain:?}: {reason:?}")]
    ClaimError { domain: String, reason: String },
    #[error("Could not release domain {domain:?}: {reason:?}")]
    ReleaseError { domain: String, reason: String },
    #[error("Internal registry Error: `{0}`")]
    Internal(String),
}
impl From<String> for RegistryError {
    fn from(s: String) -> Self {
        RegistryError::Internal(s)
    }
}
