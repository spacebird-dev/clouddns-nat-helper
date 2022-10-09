use std::net::{Ipv4Addr, SocketAddr};

/* the domain crate does have DNS resolving builtin, we could switch to that in the future */
use dnsclient::{sync::DNSClient, UpstreamServer};

use super::{Ipv4Source, SourceError};

/// A simple Ipv4 address source that looks up the A record for a given hostname and returns it.
///
/// Note that this source will simply return the first A record that it finds, round-robin DNS and similar
/// setups are therefore not supported.
///
/// This source does not perform any sort of caching, each call to [`Ipv4Source::addr()`] will lookup the hostname again.
///
/// To create a new source, use the [`HostnameSource::from_config()`] function
#[derive(Debug)]
#[non_exhaustive]
pub struct HostnameSource {
    hostname: String,
    client: DNSClient,
}

/// Configuration for [`HostnameSource`]. Must be supplied when creating a [`HostnameSource`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HostnameSourceConfig {
    /// The hostname to look up
    pub hostname: String,
    /// A list of DNS server addresses (IP address + Port number) to use for looking up the hostname.
    pub servers: Vec<SocketAddr>,
}

impl Ipv4Source for HostnameSource {
    fn addr(&self) -> Result<Ipv4Addr, SourceError> {
        match self.client.query_a(&self.hostname) {
            Ok(addrs) => match addrs.get(0) {
                Some(a) => Ok(a.to_owned()),
                None => Err(SourceError {
                    msg: format!(
                        "query for host {} did not return an IPv4 address",
                        self.hostname
                    ),
                }),
            },
            Err(e) => Err(e.to_string().into()),
        }
    }
}

impl HostnameSource {
    /// Create a new [`HostnameSource`] with the supplied configuration.
    /// Returns an error if the initialization of the source fails
    pub fn from_config(config: &HostnameSourceConfig) -> Result<Box<dyn Ipv4Source>, SourceError> {
        let client = DNSClient::new(
            config
                .servers
                .iter()
                .copied()
                .map(UpstreamServer::new)
                .collect(),
        );
        let source = HostnameSource {
            hostname: config.hostname.to_owned(),
            client,
        };
        match source.addr() {
            Ok(_) => Ok(Box::new(source)),
            Err(e) => Err(format!(
                "could not initialize HostnameSource (maybe your hostname is invalid)?: {}",
                e
            )
            .into()),
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn should_return_ip_address() {
        panic!()
    }
}
