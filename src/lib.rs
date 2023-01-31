//! Main crate for the `clouddns_nat_helper` application.
//!
//! For usage information, see: [here](https://github.com/maxhoesel/clouddns-nat-helper)
//!
//! For more information, choose one of the modules below.
//! The following modules might be of interest if you want to add new functionality:
//! - [`ipv4source`]s are used to retrieve a valid Ipv4 address for any managed A records
//! - [`provider`]s are DNS providers such as Cloudflare that ultimately server DNS records to clients
//! - [`registry`] is used to implement ownership over DNS A records, preventing conflicts with other instances of this application

#![allow(clippy::uninlined_format_args)]

pub mod ipv4source;
pub mod plan;
pub mod provider;
pub mod registry;
