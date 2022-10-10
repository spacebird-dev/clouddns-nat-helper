#![allow(non_camel_case_types)]
use std::fmt::{write, Display};

use clap::ValueEnum;
use log::LevelFilter;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, ValueEnum)]
pub enum Ipv4AddressSource {
    Hostname,
    Fixed,
}

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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, ValueEnum)]
pub enum Policy {
    CreateOnly,
    Upsert,
    Sync,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, ValueEnum)]
pub enum Provider {
    Cloudflare,
}

pub type TTL = u32;
