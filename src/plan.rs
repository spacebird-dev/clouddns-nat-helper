use std::{collections::HashMap, net::Ipv4Addr};

use log::{info, trace, warn};

use crate::{
    config::Policy,
    provider::DnsRecord,
    registry::{ARegistry, Domain},
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
    fn create_a_action(name: String, addr: &Ipv4Addr) -> DnsRecord {
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
                if !current.aaaa.is_empty() {
                    // There is at least one AAAA record, this domain needs to up-to-date
                    if current.a.is_empty() {
                        info!(
                            "No A record found for owned domain {}, creating",
                            current.name
                        );
                        plan.create_actions.push(Plan::create_a_action(
                            domain.name.to_owned(),
                            desired_address,
                        ));
                    } else if current.a.contains(desired_address) && current.a.len() == 1 {
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
                    } else {
                        info!("Found outdated A record(s) for domain {}, but policy is {:?}, not modifying. Records: {:?}", current.name, policy, current.a);
                    }
                } else if matches!(policy, Policy::Sync) {
                    info!(
                        "No more AAAA records associated with owned domain {}, deleting",
                        domain.name
                    );
                    plan.delete_actions.extend(Plan::delete_a_actions(domain));
                } else {
                    info!("No more AAAA records associated with owned domain {}, but policy is {:?}, not modifying", current.name, policy);
                }
            } else if !domain.aaaa.is_empty() && domain.a.is_empty() {
                // Domain not owned and matches our criteria (at least one AAAA record and no A records), see if we can claim it
                match registry.claim(domain.name.as_str()) {
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
        plan
    }
}

#[cfg(test)]
mod tests {
    use std::{
        net::{Ipv4Addr, Ipv6Addr},
        vec,
    };

    use totems::assert_contains;

    use crate::{
        provider::DnsRecord,
        registry::{ARegistry, Domain, MockARegistry},
    };

    use super::Plan;

    static DESIRED_IP: Ipv4Addr = Ipv4Addr::new(10, 10, 10, 10);
    fn owned_correct_d() -> Domain {
        Domain {
            name: "owned-ok.example.com".to_string(),
            a: vec![DESIRED_IP],
            aaaa: vec![Ipv6Addr::new(0xfd42, 1, 1, 1, 1, 1, 1, 3)],
            txt: vec!["i_am_tenant".to_string()],
            a_ownership: crate::registry::Ownership::Owned,
        }
    }
    fn owned_to_insert_d() -> Domain {
        Domain {
            name: "owned-but-no-a.example.com".to_string(),
            a: vec![],
            aaaa: vec![Ipv6Addr::new(0xfd42, 1, 1, 1, 1, 1, 1, 3)],
            txt: vec!["i_am_tenant".to_string()],
            a_ownership: crate::registry::Ownership::Owned,
        }
    }
    fn owned_to_update_d() -> Domain {
        Domain {
            name: "owned-but-old-a.example.com".to_string(),
            a: vec![Ipv4Addr::new(10, 10, 10, 111)],
            aaaa: vec![Ipv6Addr::new(0xfd42, 1, 1, 1, 1, 1, 1, 3)],
            txt: vec!["i_am_tenant".to_string()],
            a_ownership: crate::registry::Ownership::Owned,
        }
    }
    fn owned_multiple_a_with_correct_d() -> Domain {
        Domain {
            name: "owned-but-multiple-a-with-correct.example.com".to_string(),
            a: vec![DESIRED_IP, Ipv4Addr::new(10, 10, 10, 111)],
            aaaa: vec![Ipv6Addr::new(0xfd42, 1, 1, 1, 1, 1, 1, 3)],
            txt: vec!["i_am_tenant".to_string()],
            a_ownership: crate::registry::Ownership::Owned,
        }
    }
    fn owned_multiple_a_without_correct_d() -> Domain {
        Domain {
            name: "owned-but-multiple-a-without-correct.example.com".to_string(),
            a: vec![
                Ipv4Addr::new(10, 10, 10, 111),
                Ipv4Addr::new(10, 10, 10, 123),
            ],
            aaaa: vec![Ipv6Addr::new(0xfd42, 1, 1, 1, 1, 1, 1, 3)],
            txt: vec!["i_am_tenant".to_string()],
            a_ownership: crate::registry::Ownership::Owned,
        }
    }
    fn owned_to_delete_incorrect_a_d() -> Domain {
        Domain {
            name: "owned-but-to-delete-and-old-a.example.com".to_string(),
            a: vec![Ipv4Addr::new(10, 1, 1, 1)],
            aaaa: vec![],
            txt: vec!["i_am_tenant".to_string()],
            a_ownership: crate::registry::Ownership::Owned,
        }
    }
    fn owned_to_delete_correct_a_d() -> Domain {
        Domain {
            name: "owned-but-to-delete.example.com".to_string(),
            a: vec![DESIRED_IP],
            aaaa: vec![],
            txt: vec!["i_am_tenant".to_string()],
            a_ownership: crate::registry::Ownership::Owned,
        }
    }
    fn owned_to_delete_multiple_a_with_correct_d() -> Domain {
        Domain {
            name: "owned-but-to-delete-multiple-a-with-correct.example.com".to_string(),
            a: vec![DESIRED_IP, Ipv4Addr::new(10, 1, 1, 1)],
            aaaa: vec![],
            txt: vec!["i_am_tenant".to_string()],
            a_ownership: crate::registry::Ownership::Owned,
        }
    }
    fn owned_to_delete_multiple_a_without_correct_d() -> Domain {
        Domain {
            name: "owned-but-to-delete-multiple-a-without-correct.example.com".to_string(),
            a: vec![DESIRED_IP, Ipv4Addr::new(10, 1, 1, 1)],
            aaaa: vec![],
            txt: vec!["i_am_tenant".to_string()],
            a_ownership: crate::registry::Ownership::Owned,
        }
    }
    fn available_d() -> Domain {
        Domain {
            name: "available.example.com".to_string(),
            aaaa: vec![Ipv6Addr::new(0xfd42, 1, 1, 1, 1, 1, 1, 1)],
            a: vec![],
            txt: vec![],
            a_ownership: crate::registry::Ownership::Available,
        }
    }
    fn taken_d() -> Domain {
        Domain {
            name: "taken.example.com".to_string(),
            a: vec![Ipv4Addr::new(10, 1, 1, 2)],
            aaaa: vec![],
            txt: vec![],
            a_ownership: crate::registry::Ownership::Taken,
        }
    }

    fn mock() -> Box<dyn ARegistry> {
        let mut mock = MockARegistry::new();
        mock.expect_all_domains().returning(|| {
            vec![
                owned_correct_d(),
                owned_to_insert_d(),
                owned_to_update_d(),
                owned_multiple_a_with_correct_d(),
                owned_multiple_a_without_correct_d(),
                owned_to_delete_incorrect_a_d(),
                owned_to_delete_correct_a_d(),
                owned_to_delete_multiple_a_with_correct_d(),
                owned_to_delete_multiple_a_without_correct_d(),
                available_d(),
                taken_d(),
            ]
        });
        mock.expect_owned_domains().returning(|| {
            vec![
                owned_correct_d(),
                owned_to_insert_d(),
                owned_to_update_d(),
                owned_multiple_a_with_correct_d(),
                owned_multiple_a_without_correct_d(),
                owned_to_delete_incorrect_a_d(),
                owned_to_delete_correct_a_d(),
                owned_to_delete_multiple_a_with_correct_d(),
                owned_to_delete_multiple_a_without_correct_d(),
            ]
        });
        mock.expect_claim()
            .withf(|name| name == &available_d().name.as_str())
            .return_const(Ok(()));
        Box::new(mock)
    }

    #[test]
    fn should_generate_valid_plan_sync() {
        let plan = Plan::generate(mock().as_mut(), &DESIRED_IP, &crate::config::Policy::Sync);

        let create_must_contain = vec![
            DnsRecord {
                domain_name: owned_to_insert_d().name,
                content: crate::provider::RecordContent::A(DESIRED_IP),
            },
            DnsRecord {
                domain_name: owned_to_update_d().name,
                content: crate::provider::RecordContent::A(DESIRED_IP),
            },
            DnsRecord {
                domain_name: available_d().name,
                content: crate::provider::RecordContent::A(DESIRED_IP),
            },
            DnsRecord {
                domain_name: owned_multiple_a_without_correct_d().name,
                content: crate::provider::RecordContent::A(DESIRED_IP),
            },
        ];
        // If an owned domain somehow contains multiple A records, one of which is valid, that record could be deleted and recreated,
        // or the tests can leave it alone. Either option is valid
        let may_delete_and_recreate = vec![DnsRecord {
            domain_name: owned_multiple_a_with_correct_d().name,
            content: crate::provider::RecordContent::A(DESIRED_IP),
        }];
        let delete_must_contain = vec![
            // Providers do not implement an "update" method, so updating an existing record involves recreating it
            DnsRecord {
                domain_name: owned_to_update_d().name,
                content: crate::provider::RecordContent::A(owned_to_update_d().a.remove(0)),
            },
            // All the incorrect A records need to be deleted
            DnsRecord {
                domain_name: owned_multiple_a_with_correct_d().name,
                content: crate::provider::RecordContent::A(
                    owned_multiple_a_with_correct_d().a.remove(1),
                ),
            },
            DnsRecord {
                domain_name: owned_multiple_a_without_correct_d().name,
                content: crate::provider::RecordContent::A(
                    owned_multiple_a_without_correct_d().a.remove(0),
                ),
            },
            DnsRecord {
                domain_name: owned_multiple_a_without_correct_d().name,
                content: crate::provider::RecordContent::A(
                    owned_multiple_a_without_correct_d().a.remove(1),
                ),
            },
            // Standard deletes if no more AAAA record is present
            DnsRecord {
                domain_name: owned_to_delete_correct_a_d().name,
                content: crate::provider::RecordContent::A(DESIRED_IP),
            },
            DnsRecord {
                domain_name: owned_to_delete_incorrect_a_d().name,
                content: crate::provider::RecordContent::A(
                    owned_to_delete_incorrect_a_d().a.remove(0),
                ),
            },
            // Needs to delete both the incorrect and the correct A records
            DnsRecord {
                domain_name: owned_to_delete_multiple_a_with_correct_d().name,
                content: crate::provider::RecordContent::A(
                    owned_to_delete_multiple_a_with_correct_d().a.remove(0),
                ),
            },
            DnsRecord {
                domain_name: owned_to_delete_multiple_a_with_correct_d().name,
                content: crate::provider::RecordContent::A(
                    owned_to_delete_multiple_a_with_correct_d().a.remove(1),
                ),
            },
            DnsRecord {
                domain_name: owned_to_delete_multiple_a_without_correct_d().name,
                content: crate::provider::RecordContent::A(
                    owned_to_delete_multiple_a_without_correct_d().a.remove(0),
                ),
            },
            DnsRecord {
                domain_name: owned_to_delete_multiple_a_without_correct_d().name,
                content: crate::provider::RecordContent::A(
                    owned_to_delete_multiple_a_without_correct_d().a.remove(1),
                ),
            },
        ];

        // Check that all the required records are present
        for r in &create_must_contain {
            assert_contains!(&plan.create_actions, r);
        }
        for r in &delete_must_contain {
            assert_contains!(&plan.delete_actions, r);
        }

        // Check that there are no other records that snuck into the plan
        for r in &plan.create_actions {
            if !create_must_contain.contains(r) {
                // Some records may be deleted and recreated, ensure that they are present in both actions
                assert_contains!(&may_delete_and_recreate, r);
                assert_contains!(&plan.delete_actions, r);
            }
        }
        for r in &plan.delete_actions {
            if !delete_must_contain.contains(r) {
                // Some records may be deleted and recreated, ensure that they are present in both actions
                assert_contains!(&may_delete_and_recreate, r);
                assert_contains!(&plan.create_actions, r);
            }
        }
    }

    #[test]
    fn should_generate_valid_plan_create_only() {
        let plan = Plan::generate(
            mock().as_mut(),
            &DESIRED_IP,
            &crate::config::Policy::CreateOnly,
        );

        let create_must_contain = vec![
            DnsRecord {
                domain_name: owned_to_insert_d().name,
                content: crate::provider::RecordContent::A(DESIRED_IP),
            },
            DnsRecord {
                domain_name: available_d().name,
                content: crate::provider::RecordContent::A(DESIRED_IP),
            },
        ];

        // Check that all the required records are present
        // Poor mans order-independent equivalence check
        for r in &create_must_contain {
            assert_contains!(&plan.create_actions, r);
        }
        for r in &plan.create_actions {
            assert_contains!(&create_must_contain, r);
        }
    }

    #[test]
    fn should_generate_valid_plan_update() {
        let plan = Plan::generate(mock().as_mut(), &DESIRED_IP, &crate::config::Policy::Upsert);

        let create_must_contain = vec![
            DnsRecord {
                domain_name: owned_to_insert_d().name,
                content: crate::provider::RecordContent::A(DESIRED_IP),
            },
            DnsRecord {
                domain_name: owned_to_update_d().name,
                content: crate::provider::RecordContent::A(DESIRED_IP),
            },
            DnsRecord {
                domain_name: available_d().name,
                content: crate::provider::RecordContent::A(DESIRED_IP),
            },
            DnsRecord {
                domain_name: owned_multiple_a_without_correct_d().name,
                content: crate::provider::RecordContent::A(DESIRED_IP),
            },
        ];
        // If an owned domain somehow contains multiple A records, one of which is valid, that record could be deleted and recreated,
        // or the plan can leave it alone. Either option is valid
        let may_delete_and_recreate = vec![DnsRecord {
            domain_name: owned_multiple_a_with_correct_d().name,
            content: crate::provider::RecordContent::A(DESIRED_IP),
        }];
        let delete_must_contain = vec![
            // Providers do not implement an "update" method, so updating an existing record involves recreating it
            DnsRecord {
                domain_name: owned_to_update_d().name,
                content: crate::provider::RecordContent::A(owned_to_update_d().a.remove(0)),
            },
            // All the incorrect A records need to be deleted
            DnsRecord {
                domain_name: owned_multiple_a_with_correct_d().name,
                content: crate::provider::RecordContent::A(
                    owned_multiple_a_with_correct_d().a.remove(1),
                ),
            },
            DnsRecord {
                domain_name: owned_multiple_a_without_correct_d().name,
                content: crate::provider::RecordContent::A(
                    owned_multiple_a_without_correct_d().a.remove(0),
                ),
            },
            DnsRecord {
                domain_name: owned_multiple_a_without_correct_d().name,
                content: crate::provider::RecordContent::A(
                    owned_multiple_a_without_correct_d().a.remove(1),
                ),
            },
        ];

        // Check that all the required records are present
        for r in &create_must_contain {
            assert_contains!(&plan.create_actions, r);
        }
        for r in &delete_must_contain {
            assert_contains!(&plan.delete_actions, r);
        }

        // Check that there are no other records that snuck into the plan
        for r in &plan.create_actions {
            if !create_must_contain.contains(r) {
                // Some records may be deleted and recreated, ensure that they are present in both actions
                assert_contains!(&may_delete_and_recreate, r);
                assert_contains!(&plan.delete_actions, r);
            }
        }
        for r in &plan.delete_actions {
            if !delete_must_contain.contains(r) {
                // Some records may be deleted and recreated, ensure that they are present in both actions
                assert_contains!(&may_delete_and_recreate, r);
                assert_contains!(&plan.create_actions, r);
            }
        }
    }
}
