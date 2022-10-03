mod util;
mod wrapper;

use log::{info, trace};

use crate::{
    plan::{Action, Plan},
    types,
};

use super::{Provider, ProviderError};
use wrapper::CloudflareWrapper;

pub struct CloudflareProvider {
    api: CloudflareWrapper,
    ttl: Option<u32>,
    proxied: Option<bool>,
}

struct CloudflareProviderConfig<'a> {
    api_token: &'a str,
    ttl: Option<u32>,
    proxied: Option<bool>,
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
    fn read_records(&self) -> Result<Vec<types::DnsRecord>, ProviderError> {
        info!("Reading zones from Cloudflare API");
        let zones = self.api.list_zones()?.result;
        trace!("Collected zones {:?}", zones);

        let records = zones
            .iter()
            .map(|z| self.api.list_records(&z.id, None, None))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flat_map(|f| f.result)
            .filter_map(|r| types::DnsRecord::try_from(&r).ok())
            .collect::<Vec<types::DnsRecord>>();
        trace!("Collected Records: {:?}", records);
        Ok(records)
    }

    fn apply_plan(&self, plan: Plan) -> Vec<Result<(), ProviderError>> {
        let perform_action = |action: Action| -> Result<(), ProviderError> {
            match action {
                Action::Create(r) => {
                    info!("Performing action CREATE for Record: {:?}", r);
                    self.api
                        .create_record(
                            &self
                                .api
                                .find_record_zone(&r)
                                .ok_or(format!("Could not find suitable zone for record {:?}", r))?
                                .id,
                            &r.name,
                            &self.ttl,
                            &self.proxied,
                            r.content.into(),
                        )
                        .map(|_| ())
                        .map_err(|e| e.into())
                }
                Action::Update(r) => {
                    info!("Performing action UPDATE for Record: {:?}", r);
                    self.api
                        .update_record(
                            &self
                                .api
                                .find_record_zone(&r)
                                .ok_or(format!("Could not find suitable zone for record {:?}", r))?
                                .id,
                            &self
                                .api
                                .find_record_endpoint(&r)
                                .ok_or(format!("Could not find suitable zone for record {:?}", r))?
                                .id,
                            &r.name,
                            &self.ttl,
                            &self.proxied,
                            r.content.into(),
                        )
                        .map_err(|e| e.into())
                        .map(|_| ())
                }
                Action::Delete(r) => {
                    info!("Performing action DELETE for Record: {:?}", r);
                    self.api
                        .delete_record(
                            &self
                                .api
                                .find_record_zone(&r)
                                .ok_or(format!("Could not find suitable zone for record {:?}", r))?
                                .id,
                            &self
                                .api
                                .find_record_endpoint(&r)
                                .ok_or(format!("Could not find suitable zone for record {:?}", r))?
                                .id,
                        )
                        .map_err(|e| e.into())
                        .map(|_| ())
                }
            }
        };

        if plan.actions.is_empty() {
            info!("Empty plan, nothing to do");
            return Vec::new();
        }

        let res = plan.actions.into_iter().map(perform_action).collect();
        info!("All actions performed");
        res
    }
}
