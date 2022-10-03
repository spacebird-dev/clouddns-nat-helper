use std::net::Ipv4Addr;

use domain::base::net;

pub enum Ipv4AddressSource {
    Ipify,
    Static { address: net::Ipv4Addr },
}

pub enum Policy {
    CreateOnly,
    Upsert,
}

pub struct Config {
    pub source: Ipv4AddressSource,
    pub policy: Policy,

    pub record_ttl: Option<u32>,

    pub cloudflare_api_token: String,
    pub cloudflare_proxied: Option<bool>,

    pub fixed_address: Option<Ipv4Addr>,
}

impl Config {
    fn default() {}

    fn new(/* Get input here somehow*/) {}
}
