mod traits;
mod wrapper;

use log::{debug, trace};
use mockall_double::double;

use super::{DnsRecord, Provider, ProviderError};
use crate::{config::TTL, plan::Plan};

#[double]
use wrapper::CloudflareWrapper;

/// A [`Provider`] connecting to the Cloudflare API for creating, retrieving and deleting DNS records.
///
/// To create a provider, use the [`CloudflareProvider::from_config()`] function.
#[non_exhaustive]
pub struct CloudflareProvider {
    api: CloudflareWrapper,
    ttl: Option<TTL>,
    proxied: Option<bool>,
    dry_run: bool,
}

/// Configuration object for a [`CloudflareProvider`]. Must be supplied when creating a provider.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CloudflareProviderConfig<'a> {
    /// The API token to authenticate with. API key login is not supported
    pub api_token: &'a str,
    /// Whether newly created records should be proxied through Cloudflares protective network
    pub proxied: Option<bool>,
}

impl CloudflareProvider {
    #[cfg(not(test))]
    pub fn from_config(
        config: &CloudflareProviderConfig,
    ) -> Result<Box<dyn Provider>, ProviderError> {
        let api = CloudflareWrapper::try_new(config.api_token)?;

        Ok(Box::new(CloudflareProvider {
            api,
            ttl: None,
            proxied: config.proxied,
            dry_run: false,
        }))
    }

    #[cfg(test)]
    // Testing-only constructor, this allows us to use a mocked Wrapper in the tests
    fn from_mock_wrapper(
        config: &CloudflareProviderConfig,
        wrapper: CloudflareWrapper,
    ) -> Box<dyn Provider> {
        Box::new(CloudflareProvider {
            api: wrapper,
            ttl: None,
            proxied: config.proxied,
            dry_run: false,
        })
    }

    fn create_record(&self, rec: DnsRecord) -> Result<(), ProviderError> {
        let zone_id = &self
            .api
            .find_record_zone(&rec)
            .ok_or(format!("Could not find suitable zone for record {}", rec))?
            .id;

        if !self.dry_run {
            self.api
                .create_record(
                    zone_id,
                    &rec.domain_name,
                    &self.ttl,
                    &self.proxied,
                    rec.content.to_owned().into(),
                )
                .map_err(|e| ProviderError { msg: e.to_string() })?;
        }
        debug!("Created record {} in zone {}", rec, zone_id);
        Ok(())
    }

    fn delete_record(&self, rec: DnsRecord) -> Result<(), ProviderError> {
        let zone_id = &self
            .api
            .find_record_zone(&rec)
            .ok_or(format!("Could not find suitable zone for record {}", rec))?
            .id;
        let record_id = &self
            .api
            .find_record_endpoint(&rec)
            .ok_or(format!(
                "Could not find matching record id for record {}",
                rec
            ))?
            .id;

        if !self.dry_run {
            self.api
                .delete_record(zone_id, record_id)
                .map_err(|e| ProviderError { msg: e.to_string() })?;
        }
        debug!(
            "Deleted record {} with id {} from zone {}",
            rec, record_id, zone_id
        );
        Ok(())
    }
}

impl Provider for CloudflareProvider {
    fn records(&self) -> Result<Vec<DnsRecord>, ProviderError> {
        debug!("Reading zones from Cloudflare API");
        let zones = self.api.list_zones()?.result;
        trace!("Collected zones {:?}", zones);

        let records = zones
            .iter()
            .map(|z| self.api.list_records(&z.id))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flat_map(|f| f.result)
            .filter_map(|r| DnsRecord::try_from(&r).ok())
            .collect::<Vec<DnsRecord>>();
        trace!("Collected Records: {:?}", records);
        Ok(records)
    }

    fn apply_plan(&self, plan: Plan) -> Vec<Result<(), ProviderError>> {
        let mut results: Vec<Result<(), ProviderError>> = Vec::new();

        for rec in plan.create_actions {
            results.push(self.create_record(rec));
        }
        debug!("All create actions performed");
        for rec in plan.delete_actions {
            results.push(self.delete_record(rec));
        }
        debug!("All delete actions performed");
        results
    }

    fn supports_dry_run(&self) -> bool {
        true
    }

    fn set_dry_run(&mut self, dry_run: bool) {
        self.dry_run = dry_run;
    }

    fn ttl(&self) -> Option<TTL> {
        self.ttl
    }

    fn set_ttl(&mut self, ttl: TTL) {
        self.ttl = Some(ttl);
    }

    fn create_txt_record(&self, domain: String, content: String) -> Result<(), ProviderError> {
        self.create_record(DnsRecord {
            domain_name: domain,
            content: super::RecordContent::Txt(content),
        })
    }

    fn delete_txt_record(&self, domain: String, content: String) -> Result<(), ProviderError> {
        self.delete_record(DnsRecord {
            domain_name: domain,
            content: super::RecordContent::Txt(content),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::{net::Ipv4Addr, vec};

    use cloudflare::{
        endpoints::{self, account::AccountDetails},
        framework::response::ApiSuccess,
    };

    use super::*;

    fn zone() -> endpoints::zone::Zone {
        endpoints::zone::Zone {
            id: "123456".to_string(),
            name: "example.com".to_string(),
            account: AccountDetails {
                id: "abc123".to_string(),
                name: "Test Account".to_string(),
            },
            betas: None,
            created_on: chrono::offset::Utc::now(),
            deactivation_reason: None,
            development_mode: 0,
            host: None,
            meta: endpoints::zone::Meta {
                custom_certificate_quota: 0,
                page_rule_quota: 0,
                phishing_detected: false,
                multiple_railguns_allowed: false,
            },
            modified_on: chrono::offset::Utc::now(),
            name_servers: vec!["cash.ns.example.com".to_string()],
            original_dnshost: None,
            original_name_servers: None,
            original_registrar: None,
            owner: endpoints::zone::Owner::User {
                id: Some("abc123".to_string()),
                email: Some("fakeuser@example.com".to_string()),
            },
            paused: false,
            permissions: vec![],
            plan: None,
            plan_pending: None,
            status: endpoints::zone::Status::Active,
            vanity_name_servers: None,
            zone_type: endpoints::zone::Type::Full,
        }
    }
    fn endpoint() -> endpoints::dns::DnsRecord {
        endpoints::dns::DnsRecord {
            meta: endpoints::dns::Meta { auto_added: false },
            locked: false,
            name: "domain2.example.org".to_string(),
            ttl: 60,
            zone_id: "123456".to_string(),
            modified_on: chrono::offset::Utc::now(),
            created_on: chrono::offset::Utc::now(),
            proxiable: true,
            content: endpoints::dns::DnsContent::A {
                content: Ipv4Addr::new(10, 1, 1, 2),
            },
            id: "1234556".to_string(),
            proxied: false,
            zone_name: "example.org".to_string(),
        }
    }

    #[test]
    fn should_support_dry_run() {
        // We intentionally do not expect create/delete_record to be called. If those are called in dry_run mode we fucked up
        let mut mock = CloudflareWrapper::default();
        mock.expect_find_record_zone()
            .times(3)
            .returning(|_| Some(zone()));
        // this is the record returned for apply_plans delete entry
        mock.expect_find_record_endpoint()
            .return_once(|_| Some(endpoint()));

        let mut p = CloudflareProvider::from_mock_wrapper(
            &super::CloudflareProviderConfig {
                api_token: "abc",
                proxied: Some(false),
            },
            mock,
        );
        p.set_dry_run(true);
        p.create_txt_record("domain.example.org".to_string(), "hello".to_string())
            .unwrap();
        let r = p.apply_plan(Plan {
            create_actions: vec![DnsRecord {
                domain_name: "domain.example.org".to_string(),
                content: crate::provider::RecordContent::A(Ipv4Addr::new(10, 1, 1, 1)),
            }],
            delete_actions: vec![DnsRecord {
                domain_name: "domain2.example.org".to_string(),
                content: crate::provider::RecordContent::A(Ipv4Addr::new(10, 1, 1, 2)),
            }],
        });
        r.into_iter().for_each(|r| r.unwrap());
    }

    #[test]
    fn should_return_records() {
        let mut mock = CloudflareWrapper::default();
        mock.expect_list_zones().return_once(|| {
            Ok(ApiSuccess {
                result: vec![zone()],
                result_info: None,
                messages: serde_json::Value::Null,
                errors: vec![],
            })
        });
        mock.expect_list_records()
            .withf(|id| id == zone().id)
            .return_once(|_| {
                Ok(ApiSuccess {
                    result: vec![endpoint()],
                    result_info: None,
                    messages: serde_json::Value::Null,
                    errors: vec![],
                })
            });
        let p = CloudflareProvider::from_mock_wrapper(
            &super::CloudflareProviderConfig {
                api_token: "abc",
                proxied: Some(false),
            },
            mock,
        );

        assert_eq!(
            p.records(),
            Ok(vec![DnsRecord {
                domain_name: endpoint().name,
                content: crate::provider::RecordContent::A(Ipv4Addr::new(10, 1, 1, 2))
            }])
        );
    }
}
