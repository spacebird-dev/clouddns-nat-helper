//! Manage A record ownership using domain TXT records.

mod util;

use std::collections::HashMap;

use itertools::Itertools;
use log::{debug, info, warn};

use self::util::{insert_rec_into_d, txt_record_string, TXT_RECORD_IDENT};
use super::{ARegistry, Domain, Ownership, RegistryError};
use crate::provider::Provider;

/// The TxtRegistry manages ownership for each domains A record via an associated TXT record
/// containing a reference to this application.
///
/// This registry is a little special as it stores its ownership through the provider and not in some external
/// database/service. As such, it is coupled to a [`Provider`] instance and uses it to manage its ownership records.
/// This is transparent to any caller.
///
/// Changes to records are only allowed if this TXT record is present.
///
/// Domains may be claimed if there is no current A record and no ownership TXT record currently exists.
///
/// Use the [`TxtRegistry::from_provider()`] function to create a new registry using a provider.
#[non_exhaustive]
pub struct TxtRegistry<'a> {
    domains: HashMap<String, Domain>,
    tenant: String,
    provider: &'a dyn Provider,
    dry_run: bool,
}

impl TxtRegistry<'_> {
    /// Determine the current ownership status for a given domain
    fn determine_ownership(domain: &Domain, tenant: &str) -> Ownership {
        let owner_records: Vec<&String> = domain
            .txt
            .iter()
            .filter(|txt| txt.as_str().starts_with(TXT_RECORD_IDENT))
            .unique()
            .collect();

        match owner_records.len() {
            0 => {
                if domain.a.is_empty() {
                    // No A records and no ownership - we can manage this one
                    Ownership::Available
                } else {
                    // A records already present, seems like this domain is externally managed
                    Ownership::Taken
                }
            }
            1 => {
                if owner_records.contains(&&txt_record_string(tenant)) {
                    // We are the proud owner of this domain
                    Ownership::Owned
                } else {
                    // Some other instance of the nat-helper manages this
                    Ownership::Taken
                }
            }
            2.. => {
                warn!("Conflicting ownership of domain {} - extra ownership records were found:{:?}.\n Considering this domain taken", domain.name, owner_records);
                Ownership::Taken
            }
            _ => unreachable!(),
        }
    }

    /// Create a new [`TxtRegistry`] from a given provider
    /// As the TxtRegistry uses TXT records in the same zone for ownership, it needs a provider to manage ownership.
    /// This provider is also used to retrieve all records during creation
    pub fn from_provider(
        tenant: String,
        provider: &dyn Provider,
    ) -> Result<Box<dyn ARegistry + '_>, RegistryError> {
        let mut domains: HashMap<String, Domain> = HashMap::new();

        // Create a map of all domains that we will watch over
        for rec in &provider.records().map_err(|e| e.to_string())? {
            if let Some(d) = domains.get_mut(&rec.domain_name) {
                // Update an existing domain
                insert_rec_into_d(rec, d);
            } else {
                // Create a new domain and insert the record
                let mut d = Domain {
                    name: rec.domain_name.to_owned(),
                    a: Vec::new(),
                    aaaa: Vec::new(),
                    txt: Vec::new(),
                    a_ownership: Ownership::Taken, // Safe default, overwritten below
                };
                insert_rec_into_d(rec, &mut d);
                domains.insert(rec.domain_name.to_owned(), d);
            }
        }

        for domain in domains.values_mut() {
            domain.a_ownership = TxtRegistry::determine_ownership(domain, &tenant);
        }

        Ok(Box::new(TxtRegistry {
            domains,
            tenant,
            provider,
            dry_run: false,
        }))
    }
}

impl ARegistry for TxtRegistry<'_> {
    fn owned_domains(&self) -> Vec<super::Domain> {
        self.domains
            .values()
            .filter(|d| d.a_ownership == Ownership::Owned)
            .cloned()
            .collect_vec()
    }

    fn all_domains(&self) -> Vec<Domain> {
        self.domains.values().cloned().collect_vec()
    }

    fn claim(&mut self, name: &str) -> Result<(), super::RegistryError> {
        let Some(reg_d) = self.domains.get_mut(name) else {
            return Err(RegistryError::ClaimError {
                domain: name.to_string(),
                reason: "Not in registry".to_string(),
            });
        };

        match reg_d.a_ownership {
            Ownership::Owned => {
                info!(
                    "Attempted to claim domain {}, but it is already owned by us. Ignoring",
                    name
                );
                Ok(())
            }
            Ownership::Taken => Err(RegistryError::ClaimError {
                domain: name.to_string(),
                reason: "Owned by other instance".to_string(),
            }),
            Ownership::Available => {
                if !self.dry_run {
                    self.provider
                        .create_txt_record(reg_d.name.to_owned(), txt_record_string(&self.tenant))
                        .map_err(|e| RegistryError::ClaimError {
                            domain: name.to_string(),
                            reason: format!("Provider Error: {}", e),
                        })?;
                }
                reg_d.a_ownership = Ownership::Owned;
                debug!("Successfully claimed domain {}", name);
                Ok(())
            }
        }
    }

    fn release(&mut self, name: &str) -> Result<(), RegistryError> {
        let Some(reg_d) = self.domains.get_mut(name) else {
            return Err(RegistryError::ReleaseError {
                domain: name.to_string(),
                reason: "Not in registry".to_string(),
            });
        };

        match reg_d.a_ownership {
            Ownership::Owned => {
                if !self.dry_run {
                    self.provider
                        .delete_txt_record(reg_d.name.to_owned(), txt_record_string(&self.tenant))
                        .map_err(|e| RegistryError::ReleaseError {
                            domain: name.to_string(),
                            reason: format!("Provider Error: {}", e),
                        })?;
                }
                reg_d.a_ownership = Ownership::Available;
                debug!("Sucessfully released domain {}", name);
                Ok(())
            }
            Ownership::Taken => Err(RegistryError::ReleaseError {
                domain: name.to_string(),
                reason: "Owned by other instance".to_string(),
            }),
            Ownership::Available => {
                info!("Attempted to release domain {}, but it is already not owned by anyone. Ignoring", name);
                Ok(())
            }
        }
    }

    fn set_tenant(&mut self, tenant: String) {
        self.tenant = tenant;
    }

    fn enable_dry_run(&mut self) -> Result<(), RegistryError> {
        self.dry_run = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, Ipv6Addr};

    use crate::{
        provider::{DnsRecord, MockProvider, Provider, RecordContent},
        registry::Domain,
    };

    use super::{util::txt_record_string, TxtRegistry};

    static TENANT: &str = "evil;test_tennant;name";

    fn records() -> Vec<DnsRecord> {
        vec![
            DnsRecord {
                domain_name: "owned.example.com".to_string(),
                content: RecordContent::A(Ipv4Addr::new(10, 1, 1, 1)),
            },
            DnsRecord {
                domain_name: "owned.example.com".to_string(),
                content: RecordContent::Txt(txt_record_string(TENANT)),
            },
            DnsRecord {
                domain_name: "available.example.com".to_string(),
                content: RecordContent::Aaaa(Ipv6Addr::new(0xfd42, 1, 1, 1, 1, 1, 1, 1)),
            },
            DnsRecord {
                domain_name: "taken.example.com".to_string(),
                content: RecordContent::A(Ipv4Addr::new(10, 1, 1, 2)),
            },
            DnsRecord {
                domain_name: "other-owner.example.com".to_string(),
                content: RecordContent::A(Ipv4Addr::new(10, 1, 1, 3)),
            },
            DnsRecord {
                domain_name: "other-owner.example.com".to_string(),
                content: RecordContent::Txt(txt_record_string("other_tenant")),
            },
            DnsRecord {
                domain_name: "conflict.example.com".to_string(),
                content: RecordContent::Txt(txt_record_string("other_tenant")),
            },
            DnsRecord {
                domain_name: "conflict.example.com".to_string(),
                content: RecordContent::Txt(txt_record_string(TENANT)),
            },
            DnsRecord {
                domain_name: "conflict.example.com".to_string(),
                content: RecordContent::Aaaa(Ipv6Addr::new(0xfd42, 1, 1, 1, 1, 1, 1, 2)),
            },
            DnsRecord {
                domain_name: "conflict.example.com".to_string(),
                content: RecordContent::A(Ipv4Addr::new(10, 1, 1, 2)),
            },
        ]
    }
    fn owned_d() -> Domain {
        Domain {
            name: "owned.example.com".to_string(),
            a: vec![Ipv4Addr::new(10, 1, 1, 1)],
            aaaa: vec![],
            txt: vec![txt_record_string(TENANT)],
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
    fn other_owner_d() -> Domain {
        Domain {
            name: "other-owner.example.com".to_string(),
            a: vec![Ipv4Addr::new(10, 1, 1, 3)],
            aaaa: vec![],
            txt: vec![txt_record_string("other_tenant")],
            a_ownership: crate::registry::Ownership::Taken,
        }
    }
    fn conflict_d() -> Domain {
        Domain {
            name: "conflict.example.com".to_string(),
            a: vec![Ipv4Addr::new(10, 1, 1, 2)],
            aaaa: vec![Ipv6Addr::new(0xfd42, 1, 1, 1, 1, 1, 1, 2)],
            txt: vec![txt_record_string(TENANT), txt_record_string("other_tenant")],
            a_ownership: crate::registry::Ownership::Taken,
        }
    }

    #[test]
    fn detects_owned_domains() {
        let mut mock = MockProvider::new();
        mock.expect_records().return_once(|| Ok(records()));
        let provider_mock: Box<dyn Provider> = Box::new(mock);

        let rg = TxtRegistry::from_provider(TENANT.to_string(), provider_mock.as_ref()).unwrap();

        assert!(rg.owned_domains().len() == 1);
        assert_eq!(rg.owned_domains().get(0).unwrap(), &owned_d());
    }

    #[test]
    fn claims_available_domain() {
        let mut mock = MockProvider::new();
        mock.expect_records().return_once(|| Ok(records()));
        mock.expect_create_txt_record().return_once(|_, _| Ok(()));
        let provider_mock: Box<dyn Provider> = Box::new(mock);

        let mut rg =
            TxtRegistry::from_provider(TENANT.to_string(), provider_mock.as_ref()).unwrap();

        rg.claim(available_d().name.as_str()).unwrap();

        assert!(rg.owned_domains().len() == 2);
        assert!(rg.owned_domains().contains(&owned_d()));
        let mut available_d = available_d();
        available_d.a_ownership = crate::registry::Ownership::Owned;

        assert!(rg.owned_domains().contains(&available_d));
    }

    #[test]
    fn ignores_claimm_on_owned_domain() {
        let mut mock = MockProvider::new();
        mock.expect_records().return_once(|| Ok(records()));
        let provider_mock: Box<dyn Provider> = Box::new(mock);

        let mut rg =
            TxtRegistry::from_provider(TENANT.to_string(), provider_mock.as_ref()).unwrap();

        let before = rg.owned_domains();
        rg.claim(owned_d().name.as_str()).unwrap();
        let after = rg.owned_domains();

        assert_eq!(before, after);
        assert!(rg.owned_domains().len() == 1);
        assert!(rg.owned_domains().contains(&owned_d()));
    }

    #[test]
    fn errors_on_taken_domain_claim() {
        let mut mock = MockProvider::new();
        mock.expect_records().return_once(|| Ok(records()));
        let provider_mock: Box<dyn Provider> = Box::new(mock);

        let mut rg =
            TxtRegistry::from_provider(TENANT.to_string(), provider_mock.as_ref()).unwrap();

        rg.claim(taken_d().name.as_str()).unwrap_err();

        assert!(rg.owned_domains().len() == 1);
        assert!(rg.owned_domains().contains(&owned_d()));
    }

    #[test]
    fn errors_on_other_owner_domain_claim() {
        let mut mock = MockProvider::new();
        mock.expect_records().return_once(|| Ok(records()));
        let provider_mock: Box<dyn Provider> = Box::new(mock);

        let mut rg =
            TxtRegistry::from_provider(TENANT.to_string(), provider_mock.as_ref()).unwrap();

        rg.claim(other_owner_d().name.as_str()).unwrap_err();

        assert!(rg.owned_domains().len() == 1);
        assert!(rg.owned_domains().contains(&owned_d()));
    }

    #[test]
    fn releases_owned_domain() {
        let mut mock = MockProvider::new();
        mock.expect_records().return_once(|| Ok(records()));
        mock.expect_delete_txt_record().return_once(|_, _| Ok(()));
        let provider_mock: Box<dyn Provider> = Box::new(mock);

        let mut rg =
            TxtRegistry::from_provider(TENANT.to_string(), provider_mock.as_ref()).unwrap();

        rg.release(owned_d().name.as_str()).unwrap();
        assert!(rg.owned_domains().is_empty());
    }

    #[test]
    fn ignores_release_on_available() {
        let mut mock = MockProvider::new();
        mock.expect_records().return_once(|| Ok(records()));
        let provider_mock: Box<dyn Provider> = Box::new(mock);

        let mut rg =
            TxtRegistry::from_provider(TENANT.to_string(), provider_mock.as_ref()).unwrap();

        rg.release(available_d().name.as_str()).unwrap();

        assert!(rg.owned_domains().len() == 1);
        assert!(rg.owned_domains().get(0).unwrap() == &owned_d());
    }

    #[test]
    fn errors_on_other_owner_release() {
        let mut mock = MockProvider::new();
        mock.expect_records().return_once(|| Ok(records()));
        let provider_mock: Box<dyn Provider> = Box::new(mock);

        let mut rg =
            TxtRegistry::from_provider(TENANT.to_string(), provider_mock.as_ref()).unwrap();

        rg.release(other_owner_d().name.as_str()).unwrap_err();
        rg.release(taken_d().name.as_str()).unwrap_err();

        assert!(rg.owned_domains().len() == 1);
        assert!(rg.owned_domains().get(0).unwrap() == &owned_d());
    }

    #[test]
    fn errors_on_claiming_unknown_domain() {
        let mut mock = MockProvider::new();
        mock.expect_records().return_once(|| Ok(records()));
        let provider_mock: Box<dyn Provider> = Box::new(mock);

        let mut rg =
            TxtRegistry::from_provider(TENANT.to_string(), provider_mock.as_ref()).unwrap();

        rg.claim("unknown.example.com").unwrap_err();
    }

    #[test]
    fn errors_on_releasing_unknown_domain() {
        let mut mock = MockProvider::new();
        mock.expect_records().return_once(|| Ok(records()));
        let provider_mock: Box<dyn Provider> = Box::new(mock);

        let mut rg =
            TxtRegistry::from_provider(TENANT.to_string(), provider_mock.as_ref()).unwrap();

        rg.release("unknown.example.com").unwrap_err();
    }

    #[test]
    fn ignores_conflicting_domains() {
        let mut mock = MockProvider::new();
        mock.expect_records().return_once(|| Ok(records()));
        let provider_mock: Box<dyn Provider> = Box::new(mock);

        let mut rg =
            TxtRegistry::from_provider(TENANT.to_string(), provider_mock.as_ref()).unwrap();

        assert!(!rg.owned_domains().contains(&conflict_d()));

        rg.claim(conflict_d().name.as_str()).unwrap_err();
        rg.release(conflict_d().name.as_str()).unwrap_err();

        assert!(rg.owned_domains().len() == 1);
        assert!(rg.owned_domains().get(0).unwrap() == &owned_d());
    }
}
