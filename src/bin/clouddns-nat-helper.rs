mod cli;

use core::panic;
use std::net::{IpAddr, SocketAddr};

use clap::Parser;

use env_logger::Builder;
use itertools::Itertools;
use log::{error, info, trace};
use tokio::{
    task,
    time::{sleep, Duration},
};

use cloddns_nat_helper::{
    config,
    ipv4source::{self, Ipv4Source, SourceError},
    plan::Plan,
    provider::{self, Provider, ProviderError},
    registry::TxtRegistry,
};

use cli::Cli;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    Builder::new().filter_level(cli.loglevel.into()).init();

    loop {
        let job_cfg = cli.clone();

        if cli.dry_run {
            info!("Running in dry-run mode, no changes to the DNS provider will be made");
        }

        trace!("Starting worker thread");
        let r = task::spawn_blocking(|| run_job(job_cfg)).await;
        match r {
            Ok(r) => {
                if r.is_err() {
                    error!("Last task completed with errors")
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
        cloddns_nat_helper::config::Ipv4AddressSource::Hostname => {
            ipv4source::HostnameSource::from_config(&ipv4source::HostnameSourceConfig {
                hostname: cli.ipv4_hostname.to_owned().unwrap(),
                servers: cli
                    .ipv4_hostname_dns_servers
                    .iter()
                    .map(|ip4| SocketAddr::new(IpAddr::V4(ip4.to_owned()), 53))
                    .collect_vec(),
            })
        }
        cloddns_nat_helper::config::Ipv4AddressSource::Fixed => Ok(
            ipv4source::FixedSource::create(cli.ipv4_fixed_address.unwrap()),
        ),
    }
}

fn get_provider(cli: &Cli) -> Result<Box<dyn Provider>, ProviderError> {
    match cli.provider {
        config::Provider::Cloudflare => {
            provider::CloudflareProvider::from_config(&provider::CloudflareProviderConfig {
                api_token: cli.cloudflare_api_token.to_owned().unwrap().as_str(),
                proxied: Some(cli.cloudflare_proxied),
                dry_run: cli.dry_run,
            })
        }
    }
}

fn run_job(cli: Cli) -> Result<(), ()> {
    // TODO: Create the provider and source in main() and pass them to the worker instead of recreating them every time
    let provider = match get_provider(&cli) {
        Ok(p) => {
            info!("Connected to provider");
            p
        }
        Err(e) => {
            error!("Unable to create provider: {}", e.to_string());
            return Err(());
        }
    };
    let source = match get_source(&cli) {
        Ok(s) => {
            info!("Created IPv4 source");
            s
        }
        Err(e) => {
            error!("Unable to create ipv4source: {}", e.to_string());
            return Err(());
        }
    };

    let all_records = match provider.records().map_err(|e| e.to_string()) {
        Ok(r) => {
            info!("Retrieved records from provider");
            r
        }
        Err(e) => {
            error!("Could not retrieve records from provider: {}", e);
            return Err(());
        }
    };
    let target_addr = match source.addr().map_err(|e| e.to_string()) {
        Ok(a) => a,
        Err(e) => {
            error!("Could not retrieve target IPv4 address: {}", e);
            return Err(());
        }
    };

    let mut registry = TxtRegistry::create(all_records, cli.registry_tenant, provider.as_ref());
    info!("Initialized registry");

    // Calculate our plan that we will apply. This also registers domain where possible
    info!("Generating plan and registering domains...");
    let plan = Plan::generate(registry.as_mut(), &target_addr, &cli.policy);
    info!("Plan generated");
    info!("Creating the following records: {:?}", plan.create_actions);
    info!("Deleting the following records: {:?}", plan.delete_actions);

    // Plan is consumed when applying, save the stuff we need to delete for later
    let to_delete = plan.delete_actions.clone();

    if plan.create_actions.is_empty() && plan.delete_actions.is_empty() {
        info!("Nothing to do");
        return Ok(());
    }

    info!("Applying plan");
    let results = provider.apply_plan(plan);
    let errs = results.iter().filter(|r| r.is_err()).collect_vec();

    if errs.is_empty() {
        info!("Plan applied. No errors were encountered");
    } else {
        error!(
            "The following errors were encountered while appling changes: {:?}",
            errs
        );
        return Err(());
    }

    if !to_delete.is_empty() {
        info!("Releasing claims on deleted records");
        let mut release_errs = "Unable to release claims on the following domains: ".to_string();
        for r in to_delete {
            match registry.release(r.domain_name.to_owned()) {
                Ok(_) => {}
                Err(e) => {
                    error!("{}", e.to_string());
                    release_errs += r.domain_name.as_str();
                }
            }
        }
    } else {
        info!("No claims to release");
    }

    info!("Completed");
    Ok(())
}
