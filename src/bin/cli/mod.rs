use std::net::Ipv4Addr;

use clap::Parser;

use cloddns_nat_helper::config::{Ipv4AddressSource, Loglevel, Policy, Provider, TTL};

macro_rules! env_prefix {
    () => {
        "CLOUDDNS_NAT_"
    };
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Source of the IPv4 address to set in all A records
    #[arg(
        value_enum,
        short = 's',
        required = true,
        env = concat!(env_prefix!(), "SOURCE")
    )]
    pub source: Ipv4AddressSource,

    /// DNS provider to use
    #[arg(
        value_enum,
        short = 'p',
        required = true,
        default_value_t = Provider::Cloudflare,
        env = concat!(env_prefix!(), "PROVIDER")
    )]
    pub provider: Provider,

    /// Set the loglevel of the application
    #[arg(
        value_enum,
        short = 'l',
        default_value_t = Loglevel::Info,
        value_name = "LEVEL",
        env = concat!(env_prefix!(), "LOGLEVEL")
    )]
    pub loglevel: Loglevel,

    /// Time to wait between update operations in seconds
    #[arg(
        short = 'i',
        default_value_t = 60,
        env = concat!(env_prefix!(), "INTERVAL")
    )]
    pub interval: u64,

    /// What actions should be allowed
    #[arg(
        value_enum,
        long,
        default_value_t = Policy::Sync,
        env = concat!(env_prefix!(), "POLICY")
    )]
    pub policy: Policy,

    /// Do not make any changes to the DNS records, only show what would happen
    #[arg(long, short = 'd', action, default_value_t = false)]
    pub dry_run: bool,

    /// Optionally set a TTL for newly created records.
    /// Will use the provider default if no specified
    #[arg(
        long,
        value_name = "TTL",
        env = concat!(env_prefix!(), "RECORD_TTL"),
    )]
    pub record_ttl: Option<TTL>,

    /// Cloudflare API Token to authenticate with
    #[arg(
        long,
        required_if_eq("provider", "cloudflare"),
        value_name = "API_TOKEN",
        env = concat!(env_prefix!(), "CLOUDFLARE_API_TOKEN")
    )]
    // Hardcoded cloudflare, there's probably a better way to do this
    pub cloudflare_api_token: Option<String>,

    /// Set to enable proxying for the generated A records in Cloudflare
    #[arg(
        long,
        action,
        default_value_t = false,
        env = concat!(env_prefix!(), "CLOUDFLARE_PROXIED")
    )]
    pub cloudflare_proxied: bool,

    /// Ipv4 address to put into all A records when using the 'fixed` address sourc'
    #[arg(
        long,
        required_if_eq("source", "fixed"),
        value_name = "IPV4_ADDRESS",
        env = concat!(env_prefix!(), "IPV4_FIXED_ADDRESS")
    )]
    pub ipv4_fixed_address: Option<Ipv4Addr>,

    // Resolve this hostname to get the Ipv4 address to put into a records.
    // Only has an effect if 'source' == 'hostname'
    #[arg(
        long,
        required_if_eq("source", "hostname"),
        value_name = "HOSTNAME",
        env = concat!(env_prefix!(), "IPV4_HOSTNAME")
    )]
    pub ipv4_hostname: Option<String>,

    /// List of DNS servers to query when resolving 'ipv4_hostname'.
    /// Only has an effect if 'source' == 'hostname'
    #[arg(
        long,
        value_name = "SERVER_IP",
        use_value_delimiter = true,
        value_delimiter = ',',
        default_values=["8.8.8.8", "1.1.1.1"],
        env = concat!(env_prefix!(), "IPV4_HOSTNAME_DNS_SERVERS")
    )]
    pub ipv4_hostname_dns_servers: Vec<Ipv4Addr>,

    /// Unique identifier (tenant) to use in registry txt records
    #[arg(
        long,
        default_value = "default",
        value_name = "TENANT",
        env = concat!(env_prefix!(), "TXT_TENANT")
    )]
    pub txt_tenant: String,
}
