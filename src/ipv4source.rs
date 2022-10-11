//! A way to retrieve an IPv4 address for use in A records.
//! Each source implements the [`Ipv4Source`] trait.
//!
//! The following sources are currently available:
//! - [`FixedSource`]: Returns a static Ipv4 address
//! - [`HostnameSource`]: Resolves a hostname to an IPv4 address and returns it

mod fixed;
mod hostname;

pub use fixed::FixedSource;
pub use hostname::{HostnameSource, HostnameSourceConfig};

use std::{fmt::Display, net::Ipv4Addr};

/// An `Ipv4Source` can be used to retrieve a single IPv4 address for use in DNS records.
pub trait Ipv4Source {
    fn addr(&self) -> Result<Ipv4Addr, SourceError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceError {
    msg: String,
}
impl Display for SourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}
impl std::error::Error for SourceError {}
impl From<String> for SourceError {
    fn from(s: String) -> Self {
        SourceError { msg: s }
    }
}
