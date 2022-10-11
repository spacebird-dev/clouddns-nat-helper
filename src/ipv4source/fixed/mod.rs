use std::net::Ipv4Addr;

use super::{Ipv4Source, SourceError};

/// A simple [`Ipv4Source`] that always returns the same static IP address.
///
/// Create such a source with the [`FixedSource::from_addr()`] function.
#[derive(Debug)]
#[non_exhaustive]
pub struct FixedSource {
    addr: Ipv4Addr,
}
impl Ipv4Source for FixedSource {
    fn addr(&self) -> Result<Ipv4Addr, SourceError> {
        Ok(self.addr)
    }
}
impl FixedSource {
    pub fn from_addr(address: Ipv4Addr) -> Box<dyn Ipv4Source> {
        Box::new(FixedSource { addr: address })
    }
}
