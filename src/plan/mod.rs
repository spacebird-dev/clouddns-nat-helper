use std::{collections::HashMap, fmt::Display, net::Ipv4Addr};

use log::{debug, info};

use crate::{
    config::Policy,
    ipv4registry::{Domain, DomainName, Ipv4Registry},
    provider::DnsRecord,
};

#[derive(Debug)]
pub struct Plan {
    pub actions: Vec<Action>,
}

impl Plan {
    // Generate a CREATE action for a given name and address
    fn create_ipv4_action(name: DomainName, addr: &Ipv4Addr) -> Action {
        let c = Action::Create(DnsRecord {
            name,
            content: crate::provider::RecordContent::A(*addr),
            ttl: None,
        });
        debug!("{}", c);
        c
    }

    // Generate a list of DELETE actions for all Ipv4 records associated with a domain
    fn delete_ipv4_actions(domain: &Domain) -> Vec<Action> {
        domain
            .a
            .iter()
            .map(|addr| {
                Action::Delete(DnsRecord {
                    name: domain.name.to_owned(),
                    content: crate::provider::RecordContent::A(*addr),
                    ttl: None,
                })
            })
            .inspect(|a| debug!("{}", a))
            .collect()
    }

    // Generate a plan of changes to apply by querying a registry for possible new A records
    // based on a Ipv6 recod set
    pub fn generate(
        ipv6domains: Vec<DomainName>,
        registry: &dyn Ipv4Registry,
        desired_address: &Ipv4Addr,
        policy: Policy,
    ) -> Plan {
        let mut plan = Plan {
            actions: Vec::new(),
        };

        let owned = registry
            .owned_domains()
            .into_iter()
            .map(|d| (d.name.to_owned(), d))
            .collect::<HashMap<_, _>>();

        for v6name in &ipv6domains {
            if let Some(current) = owned.get(v6name) {
                // We own this domains IPv4
                if current.a.contains(desired_address) {
                    info!("No action needed for domain {}", v6name);
                    continue;
                } else if !matches!(policy, Policy::CreateOnly) {
                    // Delete old ipv4 records and push our desired address
                    info!(
                        "Found outdated A record(s) for domain {}, updating",
                        current.name
                    );
                    plan.actions.extend(Plan::delete_ipv4_actions(current));
                    plan.actions
                        .push(Plan::create_ipv4_action(v6name.to_owned(), desired_address));
                }
            } else if registry.register_domain(v6name.to_owned()).is_ok() {
                info!("Registered new AAAA domain {}", v6name);
                plan.actions
                    .push(Plan::create_ipv4_action(v6name.to_owned(), desired_address));
            }
            // We weren't able to register this domain, skip it
            debug!("Unable to register domain {}, ignoring", v6name);
        }

        // Delete domains for which there is no ipv6 address anymore
        if matches!(policy, Policy::Sync) {
            for domain in owned.values() {
                if !ipv6domains.contains(&domain.name) {
                    info!(
                        "No more AAAA records associated with domain {}, deleting",
                        domain.name
                    );
                    plan.actions.extend(Plan::delete_ipv4_actions(domain));
                }
            }
        }

        plan
    }
}

#[derive(Debug)]
pub enum Action {
    Create(DnsRecord),
    Delete(DnsRecord),
}

impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Action::Create(r) => format!("Create {}", r),
                Action::Delete(r) => format!("Delete {}", r),
            }
        )
    }
}
