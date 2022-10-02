use std::net::{Ipv4Addr, Ipv6Addr};

#[derive(Debug)]
pub struct DnsRecord {
    pub name: String,
    pub content: RecordContent,
    pub ttl: u32,
}
#[derive(Debug)]
pub enum RecordContent {
    A(Ipv4Addr),
    Aaaa(Ipv6Addr),
    Txt(String),
}
