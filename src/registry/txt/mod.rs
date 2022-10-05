// TODO: Remember to delete ownership for any domains where action is just remove
mod util;

use std::{collections::HashMap, hash::Hash};

use cloudflare::endpoints::zone::Owner;
use itertools::Itertools;
use log::{debug, warn};

use crate::provider::{DnsRecord, Provider, RecordContent};

use self::util::rec_into_reg_d;

use super::{ARegistry, Domain, DomainName};

const TXT_RECORD_IDENT: &str = "clouddns_nat";
const TXT_RECORD_SEP: &str = ";";
// Returns the TXT ownership record content for a given tenant
fn txt_record_string(tenant: &str) -> String {
    format!("{}_{}{}rec: A", TXT_RECORD_IDENT, tenant, TXT_RECORD_SEP)
}

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
}

impl TxtRegistry {
    pub fn new(records: Vec<DnsRecord>, tenant: String) -> Self {
        let tenant = tenant.replace(TXT_RECORD_SEP, "");
        let mut domains: HashMap<String, RegisteredDomain> = HashMap::new();

        // Create a map of all domains that we will watch over
        for rec in &records {
            if let Some(reg_d) = domains.get_mut(&rec.name) {
                // Update an existing domain
                rec_into_reg_d(rec, reg_d)
            } else {
                // Create a new domain and insert the record
                let mut reg_d = RegisteredDomain {
                    domain: Domain {
                        name: rec.name.to_owned(),
                        a: Vec::new(),
                        aaaa: Vec::new(),
                        txt: Vec::new(),
                    },
                    ownership: Ownership::Taken, // Safe default
                };
                rec_into_reg_d(rec, &mut reg_d);
                domains.insert(rec.name.to_owned(), reg_d);
            }
        }

        // Time to parse ownerships
        for (id, domain) in domains.iter_mut() {
            let owner_records: Vec<&String> = domain
                .domain
                .txt
                .iter()
                .filter(|txt| txt.as_str().starts_with(TXT_RECORD_IDENT))
                .unique()
                .collect();

            domain.ownership = match owner_records.len() {
                0 => {
                    if domain.domain.a.is_empty() {
                        // No A records and no ownership - we can manage this one
                        Ownership::Available
                    } else {
                        // A records already present, don't overwrite
                        Ownership::Taken
                    }
                }
                1 => {
                    if owner_records.contains(&&txt_record_string(tenant.as_str())) {
                        // We are the proud owner of this domain
                        Ownership::Owned
                    } else {
                        Ownership::Taken
                    }
                }
                2.. => {
                    warn!("Conflicting ownership of domain {} - extra ownership records were found:{:?}.\n Ignoring this domain", id, owner_records);
                    Ownership::Taken
                }
                _ => unreachable!(),
            };
        }

        TxtRegistry { domains, tenant }
    }
}

impl ARegistry for TxtRegistry {
    fn owned_domains(&self) -> Vec<super::Domain> {
        todo!()
    }

    fn register_domain(&self, name: super::DomainName) -> Result<(), super::RegistryError> {
        todo!()
    }

    fn apply_plan(&self, plan: &crate::plan::Plan, provider: &dyn crate::provider::Provider) {
        todo!()
    }

    fn set_tenant(&self, tenant: String) {
        todo!()
    }
}
