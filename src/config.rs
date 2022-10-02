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
    pub cloudflare_api_token: String,
    pub ipv4_address_source: Ipv4AddressSource,
    pub ttl: Option<u32>,
    pub cloudflare_proxied: Option<bool>,
    pub policy: Policy,
}

impl Config {
    fn default() {}

    fn new(/* Get input here somehow*/) {}
}
