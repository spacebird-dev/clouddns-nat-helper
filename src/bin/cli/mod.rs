#![allow(non_camel_case_types)]

use clap::Parser;
use clouddns_nat_helper::provider::TTL;
use std::net::Ipv4Addr;

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
        long,
        required = true,
        env = concat!(env_prefix!(), "SOURCE")
    )]
    pub source: Ipv4AddressSource,

    /// DNS provider to use
    #[arg(
        value_enum,
        short = 'p',
        long,
        required = true,
        default_value_t = Provider::Cloudflare,
        env = concat!(env_prefix!(), "PROVIDER")
    )]
    pub provider: Provider,

    /// Set the loglevel of the application
    #[arg(
        value_enum,
        short = 'l',
        long,
        default_value_t = Loglevel::Info,
        value_name = "LEVEL",
        env = concat!(env_prefix!(), "LOGLEVEL")
    )]
    pub loglevel: Loglevel,

    /// Only run the utility once, then exit
    #[arg(long, default_value_t = false, action)]
    pub run_once: bool,

    /// Time to wait between update operations in seconds
    #[arg(
        short = 'i',
        long,
        default_value_t = 60,
        env = concat!(env_prefix!(), "INTERVAL")
    )]
    pub interval: u64,

    /// What A record actions are permitted. createonly: create, upsert: create,update, sync: create,update,delete.
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

    /// Ipv4 address to put into all A records when using the 'fixed` address source
    #[arg(
        long,
        required_if_eq("source", "fixed"),
        value_name = "IPV4_ADDRESS",
        env = concat!(env_prefix!(), "IPV4_FIXED_ADDRESS"),
        conflicts_with = "ipv4_hostname"
    )]
    pub ipv4_fixed_address: Option<Ipv4Addr>,

    /// Resolve this hostname to get the Ipv4 address to put into a records.
    /// Only has an effect if 'source' == 'hostname'
    #[arg(
        long,
        required_if_eq("source", "hostname"),
        value_name = "HOSTNAME",
        env = concat!(env_prefix!(), "IPV4_HOSTNAME"),
        conflicts_with = "ipv4_fixed_address"
    )]
    pub ipv4_hostname: Option<String>,

    /// List of DNS servers to query when resolving 'ipv4_hostname', as a comma-separated string.
    /// Only has an effect if 'source' == 'hostname'
    #[arg(
        long,
        value_name = "SERVER_IP",
        use_value_delimiter = true,
        value_delimiter = ',',
        default_values=["8.8.8.8", "1.1.1.1"],
        conflicts_with = "ipv4_fixed_address",
        env = concat!(env_prefix!(), "IPV4_HOSTNAME_DNS_SERVERS")
    )]
    pub ipv4_hostname_dns_servers: Vec<Ipv4Addr>,

    /// Unique identifier (tenant) to use for the registry to identify this instance of nat-helper
    #[arg(
        long,
        default_value = "default",
        value_name = "TENANT",
        env = concat!(env_prefix!(), "REGISTRY_TENANT")
    )]
    pub registry_tenant: String,
}

use clap::ValueEnum;
use log::LevelFilter;

/// Which source to use for our Ipv4 address
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, ValueEnum)]
pub enum Ipv4AddressSource {
    Hostname,
    Fixed,
}

/// Used to set the applications loglevel
// This is essentially a re-creation of log:Level. However, that enum doesn't derive ValueEnum, so we have to do it manually here
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, ValueEnum)]
pub enum Loglevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}
impl From<Loglevel> for LevelFilter {
    fn from(ll: Loglevel) -> Self {
        match ll {
            Loglevel::Error => LevelFilter::Error,
            Loglevel::Warn => LevelFilter::Warn,
            Loglevel::Info => LevelFilter::Info,
            Loglevel::Debug => LevelFilter::Debug,
            Loglevel::Trace => LevelFilter::Trace,
        }
    }
}

/// What actions to allow
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, ValueEnum)]
pub enum Policy {
    CreateOnly,
    Upsert,
    Sync,
}
impl From<Policy> for clouddns_nat_helper::plan::Policy {
    fn from(value: Policy) -> Self {
        match value {
            Policy::CreateOnly => clouddns_nat_helper::plan::Policy::CreateOnly,
            Policy::Upsert => clouddns_nat_helper::plan::Policy::Upsert,
            Policy::Sync => clouddns_nat_helper::plan::Policy::Sync,
        }
    }
}

/// Which dns provider to use. Currently only contains Cloudflare
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, ValueEnum)]
pub enum Provider {
    Cloudflare,
}
