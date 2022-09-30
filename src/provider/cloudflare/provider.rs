use std::collections::HashMap;

use crate::provider::{Domain, Provider, ProviderError, Zone};
use cloudflare::endpoints;
use cloudflare::endpoints::dns::{DnsRecord, ListDnsRecordsParams};
use cloudflare::endpoints::zone::ListZonesParams;
use cloudflare::framework::response::ApiFailure;
use cloudflare::framework::{self, apiclient::ApiClient, HttpApiClient, HttpApiClientConfig};

const CLOUDFLARE_ZONE_PAGE_SIZE: u8 = 50;
const CLOUDFLARE_RECORD_PAGE_SIZE: u16 = 5000;

pub struct CloudflareProvider {
    client: framework::HttpApiClient,
    zone_name: Option<String>,
}

impl CloudflareProvider {
    fn zone_id_from_name(&self, zone: &Zone) -> Result<&str, ProviderError> {
        todo!()
    }
}

/*
impl CloudflareProvider {
    fn paged_request<ResultType, QueryType, BodyType>(&self, endpoint: &dyn CustomEndpoint)
    where
        ResultType: ApiResult,
        QueryType: Serialize,
        BodyType: Serialize,
    {
        let results: Vec<ApiResponse<ResultType>> = Vec::new();
        endpoint.set_page(1);

        let current_page = self.client.request(endpoint);
    }
}
*/

impl Provider for CloudflareProvider {
    fn all_zones(&self) -> Result<Vec<Zone>, ProviderError> {
        let mut zones: Vec<Zone> = Vec::new();
        let mut params = ListZonesParams {
            name: self.zone_name.clone(),
            status: None,
            page: Some(1),
            per_page: Some(CLOUDFLARE_ZONE_PAGE_SIZE.into()),
            order: None,
            direction: None,
            search_match: None,
        };

        let mut page_zones: Vec<Zone> = self
            .client
            .request(&endpoints::zone::ListZones {
                params: params.clone(),
            })
            .map_err(|e| ProviderError::from(&e.to_string()))?
            .result
            .iter()
            .map(|z| z.name.clone())
            .collect();
        zones.extend_from_slice(&page_zones);

        while page_zones.len() >= CLOUDFLARE_ZONE_PAGE_SIZE.into() {
            params.page = Some(params.page.unwrap() + 1);
            let res = self.client.request(&endpoints::zone::ListZones {
                params: params.clone(),
            });
            match res {
                Ok(r) => {
                    page_zones = r.result.iter().map(|z| z.name.clone()).collect();
                    zones.extend_from_slice(&page_zones);
                }
                Err(e) => match e {
                    ApiFailure::Error(code, _) => {
                        if code != http::StatusCode::NOT_FOUND {
                            return Err(ProviderError::from(&e.to_string()));
                        } else {
                            continue;
                        }
                    }
                    ApiFailure::Invalid(_) => return Err(ProviderError::from(&e.to_string())),
                },
            };
        }
        Ok(zones)
    }

    fn zone_domains(&self, zone: &Zone) -> Result<Vec<Domain>, ProviderError> {
        let zone_id = self.zone_id_from_name(zone)?;

        let mut records: Vec<DnsRecord> = Vec::new();
        let mut params = ListDnsRecordsParams {
            page: Some(1),
            per_page: Some(CLOUDFLARE_RECORD_PAGE_SIZE.into()),
            ..Default::default()
        };
        let page_records = self
            .client
            .request(&endpoints::dns::ListDnsRecords {
                params: params.clone(),
                zone_identifier: zone_id,
            })
            .map_err(|e| ProviderError::from(&e.to_string()))?
            .result;
        records.extend(page_records);

        let mut go = true;

        while go {
            params.page = Some(params.page.unwrap() + 1);
            let res = self.client.request(&endpoints::dns::ListDnsRecords {
                params: params.clone(),
                zone_identifier: zone_id,
            });
            match res {
                Ok(r) => {
                    records.extend(r.result);
                }
                Err(e) => match e {
                    ApiFailure::Error(code, _) => {
                        if code == http::StatusCode::NOT_FOUND {
                            go = false;
                        } else {
                            return Err(ProviderError::from(&e.to_string()));
                        }
                    }
                    ApiFailure::Invalid(_) => return Err(ProviderError::from(&e.to_string())),
                },
            }
        }

        let mut domain_map: HashMap<String, Domain> = HashMap::new();
        for r in records {
            match r.
            match domain_map.get(&r.name) {
                Some(d) => todo!(),
                None => todo!(),
            }
        }

        todo!()
    }

    fn update_domain_a_records(&self, domain: &Domain) -> Result<(), ProviderError> {
        todo!()
    }

    fn create(config: &crate::config::Config) -> Result<Box<dyn Provider>, ProviderError> {
        let res = HttpApiClient::new(
            framework::auth::Credentials::UserAuthToken {
                token: config.cloudflare_api_token.clone(),
            },
            HttpApiClientConfig::default(),
            cloudflare::framework::Environment::Production,
        );

        match res {
            Ok(c) => Ok(Box::new(CloudflareProvider {
                client: c,
                zone_name: config.cloudflare_zone_name.clone(),
            })),
            Err(e) => Err(ProviderError::from(&e.to_string())),
        }
    }
}
