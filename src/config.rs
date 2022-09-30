use domain::base::net;

pub enum Ipv4AddressSource {
    Ipify,
    Static { address: net::Ipv4Addr },
}

pub struct Config {
    pub cloudflare_api_token: String,
    pub cloudflare_zone_name: Option<String>,
    pub ipv4_address_source: Ipv4AddressSource,
}

impl Config {
    fn default() {}

    fn new(/* Get input here somehow*/) {}
}
