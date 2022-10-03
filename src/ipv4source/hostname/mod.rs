use std::net::{Ipv4Addr, SocketAddr};

use dnsclient::{sync::DNSClient, UpstreamServer};

use super::{Ipv4Source, SourceError};

pub struct HostnameSource {
    hostname: String,
    client: DNSClient,
}

pub struct HostnameSourceConfig {
    hostname: String,
    servers: Vec<SocketAddr>,
}

/* the domain crate does have DNS resolving builtin, we could switch to that in the future */

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
    fn from_config(config: HostnameSourceConfig) -> Result<Self, SourceError> {
        let client = DNSClient::new(
            config
                .servers
                .into_iter()
                .map(UpstreamServer::new)
                .collect(),
        );
        let source = HostnameSource {
            hostname: config.hostname,
            client,
        };
        match source.addr() {
            Ok(_) => Ok(source),
            Err(e) => Err(format!(
                "could not initialize HostnameSource (mabye your hostname is invalid)?: {}",
                e
            )
            .into()),
        }
    }
}
