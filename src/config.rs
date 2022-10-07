use std::net::Ipv4Addr;

use domain::base::net;

pub enum Ipv4AddressSource {
    Ipify,
    Static { address: net::Ipv4Addr },
}

pub enum Policy {
    CreateOnly,
    Upsert,
    Sync,
}

pub enum Provider {
    Cloudflare,
}

pub struct Config {
    pub source: Ipv4AddressSource,
    pub provider: Provider,
    pub policy: Policy,

    pub record_ttl: Option<u32>,

    pub cloudflare_api_token: String,
    pub cloudflare_proxied: Option<bool>,

    pub fixed_address: Option<Ipv4Addr>,

    pub txt_tenant: String,
}

impl Config {
    fn default() {}

    fn new(/* Get input here somehow*/) {}
}
