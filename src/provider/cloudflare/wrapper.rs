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

use crate::provider::{self, DnsRecord, ProviderError, RecordContent};

const CLOUDFLARE_ZONE_PAGE_SIZE: u8 = 50;
const CLOUDFLARE_RECORD_PAGE_SIZE: u16 = 5000;

pub struct CloudflareWrapper {
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
        ttl: &Option<u32>,
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

    pub fn try_new(api_token: &str) -> Result<CloudflareWrapper, provider::ProviderError> {
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
            .filter(|z| record.name == z.name || record.name.ends_with(&z.name))
            .collect::<Vec<_>>();

        zones.sort_by(|a, b| a.name.len().cmp(&b.name.len()));
        zones.pop()
    }

    pub fn find_record_endpoint(&self, record: &DnsRecord) -> Option<&endpoints::dns::DnsRecord> {
        self.cache
            .records
            .iter()
            .filter(|r| {
                r.name == record.name
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
// Note that this cache is ONLY used for the get_ methods, not the regular API calls
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
