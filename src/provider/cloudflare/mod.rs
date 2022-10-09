mod traits;

use cloudflare::{
    endpoints::{self},
    framework::{
        self,
        apiclient::ApiClient,
        auth::Credentials,
        response::{ApiFailure, ApiResponse},
        Environment, HttpApiClient, HttpApiClientConfig,
    },
};
use log::{debug, trace};

use super::{DnsRecord, Provider, ProviderError, RecordContent};
use crate::{config::TTL, plan::Plan};

const CLOUDFLARE_ZONE_PAGE_SIZE: u8 = 50;
const CLOUDFLARE_RECORD_PAGE_SIZE: u16 = 5000;

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
            .map(|z| self.api.list_records(&z.id, None, None))
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

/// Internal wrapper around the Cloudflare API. Provides some convenience features such as paged requests
struct CloudflareWrapper {
    client: framework::HttpApiClient,
    cache: FinderCache,
}

impl CloudflareWrapper {
    // Perform a paged request by repeatedly calling the provided request fun.
    // The request callback needs to accept a CloudflareProvider (handled by this method) and the current page_counter
    // page_size must match the page_size in the request. The caller is responsible for ensuring that these match
    fn paged_request<R>(
        &self,
        page_size: usize,
        request: &mut dyn FnMut(u32) -> ApiResponse<Vec<R>>,
    ) -> ApiResponse<Vec<R>> {
        let mut page_counter = 1;

        // Initial failures are never good, return quickly
        let mut response = request(page_counter)?;
        let mut current_size = response.result.len();

        while current_size >= page_size {
            page_counter += 1;
            match request(page_counter) {
                Ok(r) => {
                    current_size = r.result.len();
                    let mut previous_results = response.result;
                    response = r;
                    response.result.append(&mut previous_results);
                }
                Err(e) => match e {
                    ApiFailure::Error(code, _) => match code {
                        http::StatusCode::NOT_FOUND => return Ok(response),
                        _ => return Err(e),
                    },
                    ApiFailure::Invalid(e) => return Err(e.into()),
                },
            };
        }
        Ok(response)
    }

    pub fn list_zones(&self) -> ApiResponse<Vec<endpoints::zone::Zone>> {
        self.paged_request(
            CLOUDFLARE_ZONE_PAGE_SIZE.into(),
            &mut |page_counter: u32| {
                self.client.request(&endpoints::zone::ListZones {
                    params: endpoints::zone::ListZonesParams {
                        page: Some(page_counter),
                        per_page: Some(CLOUDFLARE_ZONE_PAGE_SIZE.into()),
                        ..Default::default()
                    },
                })
            },
        )
    }

    pub fn list_records(
        &self,
        zone_id: &str,
        name: Option<String>,
        kind: Option<endpoints::dns::DnsContent>,
    ) -> ApiResponse<Vec<endpoints::dns::DnsRecord>> {
        let mut r = self.paged_request(
            CLOUDFLARE_RECORD_PAGE_SIZE.into(),
            &mut |page_counter: u32| {
                self.client.request(&endpoints::dns::ListDnsRecords {
                    zone_identifier: zone_id,
                    params: endpoints::dns::ListDnsRecordsParams {
                        page: Some(page_counter),
                        name: name.to_owned(),
                        per_page: Some(CLOUDFLARE_RECORD_PAGE_SIZE.into()),
                        ..Default::default()
                    },
                })
            },
        )?;

        // Only return the recods of specified kind. std::mem::discriminant is used because we don't want to compare the enum *contents*,
        // just the variant.
        if let Some(selector) = kind {
            r.result.retain(|rec| {
                std::mem::discriminant(&rec.content) == std::mem::discriminant(&selector)
            });
        }

        Ok(r)
    }

    pub fn create_record(
        &self,
        zone_id: &str,
        name: &str,
        ttl: &Option<TTL>,
        proxied: &Option<bool>,
        content: endpoints::dns::DnsContent,
    ) -> ApiResponse<endpoints::dns::DnsRecord> {
        self.client.request(&endpoints::dns::CreateDnsRecord {
            zone_identifier: zone_id,
            params: endpoints::dns::CreateDnsRecordParams {
                priority: None,
                ttl: *ttl,
                proxied: *proxied,
                name,
                content,
            },
        })
    }

    pub fn delete_record(
        &self,
        zone_id: &str,
        record_id: &str,
    ) -> ApiResponse<endpoints::dns::DeleteDnsRecordResponse> {
        self.client.request(&endpoints::dns::DeleteDnsRecord {
            zone_identifier: zone_id,
            identifier: record_id,
        })
    }

    pub fn try_new(api_token: &str) -> Result<CloudflareWrapper, ProviderError> {
        let api = HttpApiClient::new(
            Credentials::UserAuthToken {
                token: api_token.into(),
            },
            HttpApiClientConfig::default(),
            Environment::Production,
        );

        match api {
            Ok(api) => {
                let mut wrapper = CloudflareWrapper {
                    client: api,
                    cache: FinderCache {
                        zones: Vec::new(),
                        records: Vec::new(),
                    },
                };
                let cache = FinderCache::try_new(&wrapper)?;
                wrapper.cache = cache;
                Ok(wrapper)
            }
            Err(e) => Err(ProviderError { msg: e.to_string() }),
        }
    }

    pub fn find_record_zone(&self, record: &DnsRecord) -> Option<&endpoints::zone::Zone> {
        let mut zones = self
            .cache
            .zones
            .iter()
            .filter(|z| record.domain_name == z.name || record.domain_name.ends_with(&z.name))
            .collect::<Vec<_>>();

        zones.sort_by(|a, b| a.name.len().cmp(&b.name.len()));
        zones.pop()
    }

    pub fn find_record_endpoint(&self, record: &DnsRecord) -> Option<&endpoints::dns::DnsRecord> {
        self.cache
            .records
            .iter()
            .filter(|r| {
                r.name == record.domain_name
                    || match &record.content {
                        RecordContent::A(a) => match &r.content {
                            endpoints::dns::DnsContent::A { content } => a == content,
                            _ => false,
                        },
                        RecordContent::Aaaa(aaaa) => match &r.content {
                            endpoints::dns::DnsContent::AAAA { content } => aaaa == content,
                            _ => false,
                        },
                        RecordContent::Txt(txt) => match &r.content {
                            endpoints::dns::DnsContent::TXT { content } => txt == content,
                            _ => false,
                        },
                    }
            })
            .take(1)
            .next()
    }
}

// In order to look up record zones and IDs, we need to search through all records/zones provided by the API.
// To hasten this process, we use a cache that is initialized on first run.
// Note that this cache is ONLY used for the get_ wrapper methods, not the regular API calls
struct FinderCache {
    zones: Vec<endpoints::zone::Zone>,
    records: Vec<endpoints::dns::DnsRecord>,
}
impl FinderCache {
    fn try_new(wrapper: &CloudflareWrapper) -> Result<FinderCache, ProviderError> {
        let zones = wrapper.list_zones()?.result;

        let records = zones
            .iter()
            .map(|z| wrapper.list_records(&z.id, None, None))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flat_map(|f| f.result)
            .collect::<Vec<endpoints::dns::DnsRecord>>();
        Ok(FinderCache { zones, records })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn should_support_dry_run() {
        panic!()
        // To test, just don't mock the wrapper methods
    }

    #[test]
    fn should_return_records() {
        panic!()
    }

    #[test]
    fn should_create_record() {
        // Read mockall input to verify
        panic!()
    }

    #[test]
    fn should_delete_record() {
        // Read mockall input to verify
        panic!()
    }

    #[test]
    fn should_apply_plan() {
        // Read number of mock calls to verify
        panic!()
    }
}
