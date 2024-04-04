//! Plan the actions required to bring domains up-to-date.

use std::{fmt::Display, net::Ipv4Addr};

use log::info;

use crate::registry::ARegistry;

pub type Domain = String;

/// A Plan is a list of [`Action`]s that can be applied to a [`crate::registry::ARegistry`] and a [`crate::provider::Provider`].
/// Plans contain the changes required to bring a provider from their current to their desired state.
///
/// To create a new plan, use [`Plan::generate()`].
#[derive(Debug, Eq, PartialEq, Clone, Hash)]
pub struct Plan(Vec<Action>);

/// Represents an action to be performed on a domain by a provider.
/// Note that an individual action may entail multiple steps!
/// For example: [`Action::DeleteAndRelease`] could require the deletion of several records if multiple A records are present.
/// Therefore, [`Action`]s do **not** represent individual record actions.
#[derive(Debug, Eq, PartialEq, Clone, Hash)]
#[non_exhaustive]
pub enum Action {
    /// Indicates that this domain is new and needs to be added.
    /// This means claiming ownership with a [`crate::registry::ARegistry`] and then applying the Action to a [`crate::provider::Provider`].
    ClaimAndUpdate(Domain, Ipv4Addr),
    /// Indicates that this domain is already owned but is out-of-date.
    Update(Domain, Ipv4Addr),
    /// Indicates that the entry for this domain should be deleted and the domain released
    DeleteAndRelease(Domain),
}
impl Display for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::ClaimAndUpdate(d, ip) => write!(f, "CREATE {} => {}", d, ip),
            Action::Update(d, ip) => write!(f, "UPDATE {} => {}", d, ip),
            Action::DeleteAndRelease(d) => write!(f, "DELETE {}", d),
        }
    }
}

/// Policies limit the types of [`Action`] that will be added when generating a [`Plan`]:
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Policy {
    /// Will only create new records, will not update existing ones (even for owned domains!).
    /// Note that the plan may still contain [`Action::Update`] for domains that are owned but do not currently have an A record.
    CreateOnly,
    /// Will create new records and update owned ones. Will not delete records for domains that no longer have an AAAA record.
    Upsert,
    /// Will perform all types of actions, including [`Action::ClaimAndUpdate`],[`Action::Update`] and [`Action::DeleteAndRelease`].
    Sync,
}

impl Plan {
    pub fn actions(&self) -> impl Iterator<Item = &Action> + '_ {
        self.0.iter()
    }

    fn add_create(&mut self, name: String, addr: Ipv4Addr) {
        self.0.push(Action::ClaimAndUpdate(name, addr));
    }

    fn add_update(&mut self, name: String, addr: Ipv4Addr) {
        self.0.push(Action::Update(name, addr));
    }

    fn add_delete(&mut self, name: String) {
        self.0.push(Action::DeleteAndRelease(name));
    }

    /// Generate a new plan and return it.
    ///
    /// # Inputs
    /// - registry: [`ARegistry`] that serves as the source of domains to evaluate
    /// - desired_address: The [`Ipv4Addr`] to insert into newly created A records
    /// - policy: [`Policy`]. Determines whether to overwrite or delete existing records.
    pub fn generate(
        registry: &mut dyn ARegistry,
        desired_address: Ipv4Addr,
        policy: Policy,
    ) -> Plan {
        let mut plan = Plan(vec![]);

        for domain in &registry.owned_domains() {
            if !domain.aaaa.is_empty() {
                if domain.a.is_empty() {
                    info!(
                        "No A record found for owned domain {}, creating",
                        domain.name
                    );
                    plan.add_update(domain.name.clone(), desired_address);
                } else if domain.a.len() == 1 && domain.a[0] == desired_address {
                    info!("Domain is already up-to-date: {}", domain.name);
                    continue;
                } else {
                    match policy {
                        Policy::CreateOnly => {
                            info!("Found outdated A record(s) for domain {}, but policy is {:?}, not modifying. Records: {:?}", domain.name, policy, domain.a);
                        }
                        Policy::Upsert | Policy::Sync => {
                            info!(
                                "Found outdated A record(s) for domain {}, updating",
                                domain.name
                            );
                            plan.add_update(domain.name.clone(), desired_address);
                        }
                    }
                }
            } else {
                match policy {
                    Policy::Sync => {
                        info!(
                            "No more AAAA records associated with owned domain {}, deleting",
                            domain.name
                        );
                        plan.add_delete(domain.name.clone());
                    }
                    Policy::Upsert | Policy::CreateOnly => {
                        info!("No more AAAA records associated with owned domain {}, but policy is {:?}, not modifying", domain.name, policy);
                    }
                }
            }
        }

        for domain in &registry.available_domains() {
            if !domain.aaaa.is_empty() && domain.a.is_empty() {
                // Domain not owned and matches our criteria (at least one AAAA record and no A records), try to create our A record
                plan.add_create(domain.name.clone(), desired_address);
            }
        }
        plan
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashSet,
        net::{Ipv4Addr, Ipv6Addr},
        vec,
    };

    use crate::{
        plan::{Action, Policy},
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
        mock.expect_available_domains()
            .returning(|| vec![available_d()]);
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
        mock.expect_taken_domains().returning(|| vec![taken_d()]);
        mock.expect_claim()
            .withf(|name| name == available_d().name.as_str())
            .return_const(Ok(()));
        Box::new(mock)
    }

    #[test]
    fn should_generate_valid_plan_sync() {
        let create_expected = [Action::ClaimAndUpdate(available_d().name, DESIRED_IP)];
        let update_expected = [
            Action::Update(owned_multiple_a_without_correct_d().name, DESIRED_IP),
            Action::Update(owned_to_insert_d().name, DESIRED_IP),
            Action::Update(owned_to_update_d().name, DESIRED_IP),
            Action::Update(owned_multiple_a_with_correct_d().name, DESIRED_IP),
        ];
        let delete_expected = [
            Action::DeleteAndRelease(owned_to_delete_correct_a_d().name),
            Action::DeleteAndRelease(owned_to_delete_incorrect_a_d().name),
            Action::DeleteAndRelease(owned_to_delete_multiple_a_with_correct_d().name),
            Action::DeleteAndRelease(owned_to_delete_multiple_a_without_correct_d().name),
        ];

        let plan = Plan::generate(mock().as_mut(), DESIRED_IP, Policy::Sync);

        assert_eq!(
            HashSet::from_iter(create_expected.iter().cloned()),
            plan.actions()
                .filter(|a| matches!(a, crate::plan::Action::ClaimAndUpdate(_, _)))
                .cloned()
                .collect::<HashSet<_>>()
        );
        assert_eq!(
            HashSet::from_iter(update_expected.iter().cloned()),
            plan.actions()
                .filter(|a| matches!(a, crate::plan::Action::Update(_, _)))
                .cloned()
                .collect::<HashSet<_>>()
        );
        assert_eq!(
            HashSet::from_iter(delete_expected.iter().cloned()),
            plan.actions()
                .filter(|a| matches!(a, crate::plan::Action::DeleteAndRelease(_)))
                .cloned()
                .collect::<HashSet<_>>()
        );
    }

    #[test]
    fn should_generate_valid_plan_create_only() {
        let create_expected = [Action::ClaimAndUpdate(available_d().name, DESIRED_IP)];
        let update_expected = [Action::Update(owned_to_insert_d().name, DESIRED_IP)];
        let delete_expected = [];

        let plan = Plan::generate(mock().as_mut(), DESIRED_IP, Policy::CreateOnly);

        assert_eq!(
            HashSet::from_iter(create_expected.iter().cloned()),
            plan.actions()
                .filter(|a| matches!(a, crate::plan::Action::ClaimAndUpdate(_, _)))
                .cloned()
                .collect::<HashSet<_>>()
        );
        assert_eq!(
            HashSet::from_iter(update_expected.iter().cloned()),
            plan.actions()
                .filter(|a| matches!(a, crate::plan::Action::Update(_, _)))
                .cloned()
                .collect::<HashSet<_>>()
        );
        assert_eq!(
            HashSet::from_iter(delete_expected.iter().cloned()),
            plan.actions()
                .filter(|a| matches!(a, crate::plan::Action::DeleteAndRelease(_)))
                .cloned()
                .collect::<HashSet<_>>()
        );
    }

    #[test]
    fn should_generate_valid_plan_upsert() {
        let create_expected = [Action::ClaimAndUpdate(available_d().name, DESIRED_IP)];
        let update_expected = [
            Action::Update(owned_multiple_a_without_correct_d().name, DESIRED_IP),
            Action::Update(owned_to_insert_d().name, DESIRED_IP),
            Action::Update(owned_to_update_d().name, DESIRED_IP),
            Action::Update(owned_multiple_a_with_correct_d().name, DESIRED_IP),
        ];
        let delete_expected = [];

        let plan = Plan::generate(mock().as_mut(), DESIRED_IP, Policy::Upsert);

        assert_eq!(
            HashSet::from_iter(create_expected.iter().cloned()),
            plan.actions()
                .filter(|a| matches!(a, crate::plan::Action::ClaimAndUpdate(_, _)))
                .cloned()
                .collect::<HashSet<_>>()
        );
        assert_eq!(
            HashSet::from_iter(update_expected.iter().cloned()),
            plan.actions()
                .filter(|a| matches!(a, crate::plan::Action::Update(_, _)))
                .cloned()
                .collect::<HashSet<_>>()
        );
        assert_eq!(
            HashSet::from_iter(delete_expected.iter().cloned()),
            plan.actions()
                .filter(|a| matches!(a, crate::plan::Action::DeleteAndRelease(_)))
                .cloned()
                .collect::<HashSet<_>>()
        );
    }
}
