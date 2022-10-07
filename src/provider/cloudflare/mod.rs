mod traits;
mod wrapper;

use log::{debug, info, trace};

use crate::plan::Plan;

use super::{DnsRecord, Provider, ProviderError};
use wrapper::CloudflareWrapper;

pub struct CloudflareProvider {
    api: CloudflareWrapper,
    ttl: Option<u32>,
    proxied: Option<bool>,
}

pub struct CloudflareProviderConfig<'a> {
    pub api_token: &'a str,
    pub ttl: Option<u32>,
    pub proxied: Option<bool>,
}

impl CloudflareProvider {
    fn from_config(config: &CloudflareProviderConfig) -> Result<Self, ProviderError> {
        let api = CloudflareWrapper::try_new(config.api_token)?;

        Ok(CloudflareProvider {
            api,
            ttl: config.ttl,
            proxied: config.proxied,
        })
    }
}

impl Provider for CloudflareProvider {
    fn records(&self) -> Result<Vec<DnsRecord>, ProviderError> {
        info!("Reading zones from Cloudflare API");
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

    fn create_record(&self, rec: DnsRecord) -> Result<(), ProviderError> {
        let r = self
            .api
            .create_record(
                &self
                    .api
                    .find_record_zone(&rec)
                    .ok_or(format!("Could not find suitable zone for record {}", rec))?
                    .id,
                &rec.domain,
                &self.ttl,
                &self.proxied,
                rec.content.to_owned().into(),
            )
            .map(|_| ())
            .map_err(|e| e.into());
        if r.is_ok() {
            info!("Created record {}", rec);
        }
        r
    }

    fn delete_record(&self, rec: DnsRecord) -> Result<(), ProviderError> {
        let r = self
            .api
            .delete_record(
                &self
                    .api
                    .find_record_zone(&rec)
                    .ok_or(format!("Could not find suitable zone for record {}", rec))?
                    .id,
                &self
                    .api
                    .find_record_endpoint(&rec)
                    .ok_or(format!(
                        "Could not find matching record id for record {}",
                        rec
                    ))?
                    .id,
            )
            .map_err(|e| e.into())
            .map(|_| ());
        if r.is_ok() {
            info!("Deleted record {}", rec);
        };
        r
    }
}
