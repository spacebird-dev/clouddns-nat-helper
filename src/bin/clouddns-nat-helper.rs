mod cli;
mod executor;

use core::panic;
use std::net::{IpAddr, SocketAddr};

use clap::Parser;

use env_logger::Builder;
use itertools::Itertools;
use log::{debug, error, info, trace};
use tokio::{
    task::{self},
    time::{sleep, Duration},
};

use clouddns_nat_helper::{
    ipv4source::{self, Ipv4Source, SourceError},
    provider::{self, Provider, ProviderError},
    registry::{ARegistry, RegistryError, TxtRegistry},
};

use cli::Cli;
use executor::Executor;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), String> {
    let cli = Cli::parse();

    Builder::new().filter_level(cli.loglevel.into()).init();

    loop {
        let job_cfg = cli.clone();

        trace!("Starting worker thread");
        let r = task::spawn_blocking(|| run_job(job_cfg)).await;
        match r {
            Ok(r) => {
                if r.is_err() {
                    error!("Last task completed with errors")
                }
                if cli.run_once {
                    return r.map_err(|_| "".to_string());
                }
            }
            Err(_) => {
                error!("Task panicked, aborting...");
                panic!();
            }
        }
        sleep(Duration::from_secs(cli.interval)).await;
    }
}

fn get_source(cli: &Cli) -> Result<Box<dyn Ipv4Source>, SourceError> {
    match cli.source {
        cli::Ipv4AddressSource::Hostname => {
            ipv4source::HostnameSource::from_config(&ipv4source::HostnameSourceConfig {
                hostname: cli.ipv4_hostname.to_owned().unwrap(),
                servers: cli
                    .ipv4_hostname_dns_servers
                    .iter()
                    .map(|ip4| SocketAddr::new(IpAddr::V4(ip4.to_owned()), 53))
                    .collect_vec(),
            })
        }
        cli::Ipv4AddressSource::Fixed => Ok(ipv4source::FixedSource::from_addr(
            cli.ipv4_fixed_address.unwrap(),
        )),
    }
}

fn get_provider(cli: &Cli) -> Result<Box<dyn Provider>, ProviderError> {
    match cli.provider {
        cli::Provider::Cloudflare => {
            match provider::CloudflareProvider::from_config(&provider::CloudflareProviderConfig {
                api_token: cli.cloudflare_api_token.to_owned().unwrap().as_str(),
                proxied: Some(cli.cloudflare_proxied),
            }) {
                Ok(p) => Ok(Box::new(p)),
                Err(e) => Err(e),
            }
        }
    }
}

fn get_registry<'a>(
    cli: &Cli,
    provider: &'a (dyn clouddns_nat_helper::provider::Provider + 'a),
) -> Result<Box<dyn ARegistry + 'a>, RegistryError> {
    // For now, there is only a single registry and that is TXT. in the future, we could match here
    TxtRegistry::from_provider(cli.registry_tenant.to_owned(), provider)
}

fn run_job(cli: Cli) -> Result<(), ()> {
    // TODO: Create the provider and source in main() and pass them to the worker instead of recreating them every time
    let mut provider = match get_provider(&cli) {
        Ok(p) => {
            info!("Connected to provider");
            p
        }
        Err(e) => {
            error!("Unable to create provider: {}", e.to_string());
            return Err(());
        }
    };
    if cli.record_ttl.is_some() {
        provider.set_ttl(cli.record_ttl.unwrap());
    }

    // Create a second provider for our TXT registry. TODO: ugly, should be able to reuse the previous provider if its TXTRegistry
    let mut reg_provider = match get_provider(&cli) {
        Ok(p) => {
            info!("Connected to provider");
            p
        }
        Err(e) => {
            error!("Unable to create provider: {}", e.to_string());
            return Err(());
        }
    };
    if cli.record_ttl.is_some() {
        reg_provider.set_ttl(cli.record_ttl.unwrap());
    }

    let source = match get_source(&cli) {
        Ok(s) => {
            debug!("Created IPv4 source");
            s
        }
        Err(e) => {
            error!("Unable to create ipv4source: {}", e.to_string());
            return Err(());
        }
    };

    let mut registry = match get_registry(&cli, provider.as_ref()) {
        Ok(r) => {
            debug!("Created TXT Registry");
            r
        }
        Err(e) => {
            error!("Could not create registry: {}", e);
            return Err(());
        }
    };
    info!("Initialized registry");

    let mut exec = match Executor::try_new(
        source.as_ref(),
        reg_provider.as_mut(),
        registry.as_mut(),
        cli.policy,
        cli.dry_run,
    ) {
        Ok(e) => e,
        Err(e) => {
            error!("Could not create executor: {}", e);
            return Err(());
        }
    };
    debug!("Initialized Executor");

    let res = match exec.run() {
        Ok(r) => r,
        Err(e) => {
            error!("Error during execution: {}", e);
            return Err(());
        }
    };

    if res.successes.is_empty() && res.failures.is_empty() {
        info!("No changes made");
        return Ok(());
    }

    match (res.successes.len(), res.failures.len()) {
        (0, 0) => info!("No changes made"),
        (1.., 0) => {
            info!(
                "Successfully applied the following changes: {:?}",
                res.successes
            );
            info!("No errors were encountered");
        }
        (0, 1..) => {
            info!(
                "Encountered Errors while applying the following changes: {:?}",
                res.failures
            );
        }
        (1.., 1..) => {
            info!(
                "Successfully applied the following changes: {:?}",
                res.successes
            );
            info!(
                "Encountered Errors while applying the following changes: {:?}",
                res.failures
            );
        }
    }

    Ok(())
}
