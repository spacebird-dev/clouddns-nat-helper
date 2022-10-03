mod cloudflare;

use std::{
    fmt::Display,
    net::{Ipv4Addr, Ipv6Addr},
};

use crate::plan::Plan;

// Generic error returned by a provider action
#[derive(Debug)]
pub struct ProviderError {
    msg: String,
}

impl Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg.as_str())
    }
}

impl std::error::Error for ProviderError {}

impl From<String> for ProviderError {
    fn from(s: String) -> Self {
        ProviderError { msg: s }
    }
}

// Providers implement a few basic methods to access their cloud DNS registry for reading and writing records
pub trait Provider {
    fn apply_plan(&self, plan: Plan) -> Vec<Result<(), ProviderError>>;
    fn records(&self) -> Result<Vec<DnsRecord>, ProviderError>;
}

#[derive(Debug)]
pub struct DnsRecord {
    pub name: String,
    pub content: RecordContent,
    pub ttl: Option<u32>,
}

impl Display for DnsRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.name, self.content)
    }
}

#[derive(Debug)]
pub enum RecordContent {
    A(Ipv4Addr),
    Aaaa(Ipv6Addr),
    Txt(String),
}

impl Display for RecordContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                RecordContent::A(a) => format!("A {}", a),
                RecordContent::Aaaa(aaaa) => format!("AAAA {}", aaaa),
                RecordContent::Txt(txt) => format!("TXT {}", txt),
            }
        )
    }
}
