use std::{collections::HashMap, net::Ipv4Addr};

use log::{info, trace};

use crate::{
    config::Policy,
    provider::DnsRecord,
    registry::{ARegistry, Domain, DomainName},
};

/// A Plan is a list of actions (create or delete) that will be applied to a provider and their DNS records.
/// Plans contain the changes required to bring a provider from their current to their desired state.
///
/// To create a new plan, you need to use [`Plan::generate()`]. Note that creating a Plan always requires a registry to check against.
/// This prevents overwriting non-owned records.
#[derive(Debug)]
#[non_exhaustive]
pub struct Plan {
    pub create_actions: Vec<DnsRecord>,
    pub delete_actions: Vec<DnsRecord>,
}

impl Plan {
    // Generate a dns record for a given name and address
    fn create_a_action(name: DomainName, addr: &Ipv4Addr) -> DnsRecord {
        let c = DnsRecord {
            domain_name: name,
            content: crate::provider::RecordContent::A(*addr),
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
                domain_name: domain.name.to_owned(),
                content: crate::provider::RecordContent::A(*addr),
            })
            .inspect(|a| trace!("Removing existing record {}", a))
            .collect()
    }

    /// Generate a new plan and return it.
    ///
    /// # Inputs
    /// - registry: The [`ARegistry`] to use for managing ownership of A records.
    ///             this is also the source of our domains to operate on
    /// - desired_address: The [`Ipv4Addr`] to insert into newly created A records
    /// - policy: [`Policy`]. Determines whether to overwrite or delete existing records.
    ///
    /// Note that generate automatically claims ownership of all available domains with the registry, before adding them to the plan.
    /// This ensures that the plan only contains actions for domains that we actually own.
    ///
    /// Also note that ownership is not released for any domains in [`Plan::delete_actions`],
    /// this needs to be done manually after the plan has been applied using [`ARegistry::release()`]
    pub fn generate(
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
        info!("Currently owned domains: {:?}", owned.keys());

        for domain in &registry.all_domains() {
            if let Some(current) = owned.get(&domain.name) {
                // We own this domains A records
                if current.a.contains(desired_address) {
                    info!("Domain is already up-to-date: {}", domain.name);
                    continue;
                } else if !matches!(policy, Policy::CreateOnly) {
                    // Delete old ipv4 records and push our desired address
                    info!(
                        "Found outdated A record(s) for domain {}, updating",
                        current.name
                    );
                    plan.delete_actions.extend(Plan::delete_a_actions(current));
                    plan.create_actions.push(Plan::create_a_action(
                        domain.name.to_owned(),
                        desired_address,
                    ));
                }
            } else if !domain.aaaa.is_empty() && domain.a.is_empty() {
                // Domain not owned and matches our criteria (at least one AAAA record and no A records), see if we can claim it
                match registry.claim(domain.name.to_owned()) {
                    Ok(_) => {
                        info!("Claimed new domain {}", domain.name);
                        plan.create_actions.push(Plan::create_a_action(
                            domain.name.to_owned(),
                            desired_address,
                        ));
                    }
                    Err(e) => {
                        info!("Unable to register domain {}: {}", domain.name, e);
                    }
                }
            }
            // Domain is not owned and does not have AAAA records, so we don't care
            trace!("Skipped domain {}", domain.name);
        }

        // Delete domains for which there is no ipv6 address anymore
        if matches!(policy, Policy::Sync) {
            for domain in owned.values() {
                if domain.aaaa.is_empty() {
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

#[cfg(test)]
mod tests {
    #[test]
    fn should_generate_valid_plan() {
        panic!()
    }
}
