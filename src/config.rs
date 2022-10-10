//! Various types to model a valid configuration for the application
//!
//! Also see the cli parameters in `src/bin/cli` for more details.

#![allow(non_camel_case_types)]

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

/// Which dns provider to use. Currently only contains Cloudflare
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, ValueEnum)]
pub enum Provider {
    Cloudflare,
}

// Record TTL alias
pub type TTL = u32;
