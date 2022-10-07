mod fixed;
mod hostname;

use std::{fmt::Display, net::Ipv4Addr};

#[derive(Debug, Clone)]
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

/// An `Ipv4Source` can be used to retrieve a single IPv4 address for use in DNS records
pub trait Ipv4Source {
    fn addr(&self) -> Result<Ipv4Addr, SourceError>;
}
