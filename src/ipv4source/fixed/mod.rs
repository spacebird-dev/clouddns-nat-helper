use std::net::Ipv4Addr;

use super::{Ipv4Source, SourceError};

#[derive(Debug)]
pub struct FixedSource {
    addr: Ipv4Addr,
}
impl Ipv4Source for FixedSource {
    fn addr(&self) -> Result<Ipv4Addr, SourceError> {
        Ok(self.addr)
    }
}
impl FixedSource {
    pub fn create(address: Ipv4Addr) -> Box<dyn Ipv4Source> {
        Box::new(FixedSource { addr: address })
    }
}
