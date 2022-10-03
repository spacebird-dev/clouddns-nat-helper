use std::{
    collections::HashMap,
    net::{Ipv4Addr, Ipv6Addr},
};

use crate::{config::Policy, provider::DnsRecord};

struct Domain {
    a: Vec<Ipv4Addr>,
    aaaa: Vec<Ipv6Addr>,
    txt: Vec<String>,
}

#[derive(Debug)]
pub struct Plan {
    pub actions: Vec<Action>,
}

impl Plan {
    // With registry, lookup owned records and get values
}

#[derive(Debug)]
pub enum Action {
    Create(DnsRecord),
    Update(DnsRecord),
    Delete(DnsRecord),
}
