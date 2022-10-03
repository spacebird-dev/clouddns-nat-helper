use std::net::Ipv4Addr;

use super::{Ipv4Source, SourceError};

pub struct FixedSource {
    addr: Ipv4Addr,
}

impl Ipv4Source for FixedSource {
    fn addr(&self) -> Result<Ipv4Addr, SourceError> {
        Ok(self.addr)
    }
}

impl FixedSource {
    fn new(address: Ipv4Addr) -> Self {
        FixedSource { addr: address }
    }
}
