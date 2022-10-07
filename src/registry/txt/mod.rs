mod util;

use std::collections::HashMap;

use itertools::Itertools;
use log::{info, warn};

use self::util::{rec_into_d, txt_record_string, TXT_RECORD_IDENT};
use super::{ARegistry, Domain, DomainName, RegistryError};
use crate::provider::{DnsRecord, Provider, RecordContent};

enum Ownership {
    /// This domains A record belongs to us
    Owned,
    /// This domains A records are managed by someone else
    Taken,
    /// This domain doesn't have A records, we can claim it
    Available,
}
struct RegisteredDomain {
    domain: Domain,
    ownership: Ownership,
}

pub struct TxtRegistry {
    domains: HashMap<DomainName, RegisteredDomain>,
    tenant: String,
    provider: Box<dyn Provider>,
}

impl TxtRegistry {
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

    pub fn new(records: Vec<DnsRecord>, tenant: String, provider: Box<dyn Provider>) -> Self {
        let tenant = tenant.replace(TXT_RECORD_IDENT, "");
        let mut domains: HashMap<String, RegisteredDomain> = HashMap::new();

        // Create a map of all domains that we will watch over
        for rec in &records {
            if let Some(reg_d) = domains.get_mut(&rec.domain) {
                // Update an existing domain
                rec_into_d(rec, &mut reg_d.domain)
            } else {
                // Create a new domain and insert the record
                let mut reg_d = RegisteredDomain {
                    domain: Domain {
                        name: rec.domain.to_owned(),
                        a: Vec::new(),
                        aaaa: Vec::new(),
                        txt: Vec::new(),
                    },
                    ownership: Ownership::Taken, // Safe default, overwritten below
                };
                rec_into_d(rec, &mut reg_d.domain);
                domains.insert(rec.domain.to_owned(), reg_d);
            }
        }

        for domain in domains.values_mut() {
            domain.ownership = TxtRegistry::determine_ownership(&domain.domain, &tenant);
        }

        TxtRegistry {
            domains,
            tenant,
            provider,
        }
    }

    fn action_record_is_valid(&self, rec: &DnsRecord) -> bool {
        if !self.domains.contains_key(&rec.domain) {
            warn!(
                "Plan contains action for unknown domain {}, dropping",
                rec.domain
            );
            return false;
        }

        match self.domains.get(&rec.domain).unwrap().ownership {
            Ownership::Owned => true,
            _ => {
                warn!(
                    "Plan wants to modify unowned domain {}, dropping",
                    rec.domain
                );
                false
            }
        }
    }
}

impl ARegistry for TxtRegistry {
    fn owned_domains(&self) -> Vec<super::Domain> {
        self.domains
            .values()
            .filter(|d| matches!(d.ownership, Ownership::Owned))
            .map(|d| d.domain.clone())
            .collect_vec()
    }

    fn claim(&mut self, name: &DomainName) -> Result<(), super::RegistryError> {
        if !self.domains.contains_key(name) {
            return Err(RegistryError {
                msg: format!("Domain {} not in registry", name),
            });
        }

        let reg_d = self.domains.get_mut(name).unwrap();
        match reg_d.ownership {
            Ownership::Owned => {
                info!(
                    "Attempted to claim domain {}, but it is already owned by us. Ignoring",
                    name
                );
                Ok(())
            }
            Ownership::Taken => Err(RegistryError {
                msg: format!("Domain {} cannot be claimed", name),
            }),
            Ownership::Available => {
                self.provider
                    .create_record(DnsRecord {
                        domain: name.clone(),
                        content: RecordContent::Txt(txt_record_string(&self.tenant)),
                        ttl: None,
                    })
                    .map_err(|e| RegistryError {
                        msg: format!("unable to claim domain {}: {}", name, e),
                    })?;
                reg_d.ownership = Ownership::Owned;
                info!("Sucessfully claimed domain {}", name);
                Ok(())
            }
        }
    }

    fn release(&mut self, name: &DomainName) -> Result<(), RegistryError> {
        if !self.domains.contains_key(name) {
            return Err(RegistryError {
                msg: format!("Domain {} not in registry", name),
            });
        }

        let reg_d = self.domains.get_mut(name).unwrap();
        match reg_d.ownership {
            Ownership::Owned => {
                self.provider
                    .delete_record(DnsRecord {
                        domain: name.clone(),
                        content: RecordContent::Txt(txt_record_string(&self.tenant)),
                        ttl: None,
                    })
                    .map_err(|e| RegistryError {
                        msg: format!("unable to release domain {}: {}", name, e),
                    })?;
                reg_d.ownership = Ownership::Owned;
                info!("Sucessfully released domain {}", name);
                Ok(())
            }
            Ownership::Taken => Err(RegistryError {
                msg: format!(
                    "Cannot release domain {} as it is owned by someone else",
                    name
                ),
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
}
