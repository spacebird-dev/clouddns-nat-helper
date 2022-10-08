use std::{collections::HashMap, net::Ipv4Addr};

use log::{debug, info, trace};

use crate::{
    config::Policy,
    provider::DnsRecord,
    registry::{ARegistry, Domain, DomainName},
};

#[derive(Debug)]
pub struct Plan {
    pub create_actions: Vec<DnsRecord>,
    pub delete_actions: Vec<DnsRecord>,
}

impl Plan {
    // Generate a dns record for a given name and address
    fn create_a_action(name: DomainName, addr: &Ipv4Addr) -> DnsRecord {
        let c = DnsRecord {
            domain: name,
            content: crate::provider::RecordContent::A(*addr),
            ttl: None,
        };
        trace!("New record: {}", c);
        c
    }

    // Generate a list of DELETE records actions for all A records associated with a domain
    fn delete_a_actions(domain: &Domain) -> Vec<DnsRecord> {
        domain
            .a
            .iter()
            .map(|addr| DnsRecord {
                domain: domain.name.to_owned(),
                content: crate::provider::RecordContent::A(*addr),
                ttl: None,
            })
            .inspect(|a| trace!("Removing existing record {}", a))
            .collect()
    }

    // Generate a plan of changes to apply by querying a registry for possible new A records
    // based on a Ipv6 recod set
    pub fn generate(
        aaaa_domains: Vec<DomainName>,
        registry: &mut dyn ARegistry,
        desired_address: &Ipv4Addr,
        policy: &Policy,
    ) -> Plan {
        let mut plan = Plan {
            create_actions: Vec::new(),
            delete_actions: Vec::new(),
        };

        let owned = registry
            .owned_domains()
            .into_iter()
            .map(|d| (d.name.to_owned(), d))
            .collect::<HashMap<_, _>>();
        debug!("Currently owned domains: {:?}", owned.keys());

        for aaaa_name in &aaaa_domains {
            trace!("Processing domain {}", aaaa_name);
            if let Some(current) = owned.get(aaaa_name) {
                // We own this domains A records
                if current.a.contains(desired_address) {
                    info!("No action needed for domain {}", aaaa_name);
                    continue;
                } else if !matches!(policy, Policy::CreateOnly) {
                    // Delete old ipv4 records and push our desired address
                    info!(
                        "Found outdated A record(s) for domain {}, updating",
                        current.name
                    );
                    plan.delete_actions.extend(Plan::delete_a_actions(current));
                    plan.create_actions
                        .push(Plan::create_a_action(aaaa_name.to_owned(), desired_address));
                }
            }
            // Domain not owned, see if we can claim it
            match registry.claim(aaaa_name) {
                Ok(_) => {
                    info!("Claimed new domain {}", aaaa_name);
                    plan.create_actions
                        .push(Plan::create_a_action(aaaa_name.to_owned(), desired_address));
                }
                Err(e) => {
                    debug!("Unable to register domain {}: {}", aaaa_name, e);
                }
            }
        }

        // Delete domains for which there is no ipv6 address anymore
        if matches!(policy, Policy::Sync) {
            for domain in owned.values() {
                if !aaaa_domains.contains(&domain.name) {
                    info!(
                        "No more AAAA records associated with domain {}, deleting",
                        domain.name
                    );
                    plan.delete_actions.extend(Plan::delete_a_actions(domain));
                }
            }
        }

        plan
    }
}
