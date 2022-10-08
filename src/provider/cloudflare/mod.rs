mod traits;
mod wrapper;

use log::{debug, trace};

use crate::{config::TTL, plan::Plan};

use super::{DnsRecord, Provider, ProviderError};
use wrapper::CloudflareWrapper;

pub struct CloudflareProvider {
    api: CloudflareWrapper,
    ttl: Option<TTL>,
    proxied: Option<bool>,
    dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CloudflareProviderConfig<'a> {
    pub api_token: &'a str,
    pub proxied: Option<bool>,
    pub dry_run: bool,
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
            dry_run: config.dry_run,
        }))
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
                    &rec.domain,
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
}
