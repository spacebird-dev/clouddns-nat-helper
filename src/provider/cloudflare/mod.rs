mod wrapper;

use cloudflare::endpoints;

use log::{debug, info, trace};

use crate::{
    plan::{Action, Plan},
    types,
};

use super::{Provider, ProviderError};
use wrapper::CloudflareWrapper;

impl TryFrom<&endpoints::dns::DnsRecord> for types::DnsRecord {
    type Error = String;

    fn try_from(r: &endpoints::dns::DnsRecord) -> Result<Self, Self::Error> {
        let converted_content = match &r.content {
            endpoints::dns::DnsContent::A { content } => types::RecordContent::A(*content),
            endpoints::dns::DnsContent::AAAA { content } => types::RecordContent::Aaaa(*content),
            endpoints::dns::DnsContent::TXT { content } => {
                types::RecordContent::Txt(content.clone())
            }
            _ => return Err(format!("Invalid record type: {:?}", r.content)),
        };
        Ok(types::DnsRecord {
            name: r.name.clone(),
            content: converted_content,
            ttl: r.ttl,
        })
    }
}

impl From<types::RecordContent> for endpoints::dns::DnsContent {
    fn from(c: types::RecordContent) -> Self {
        match &c {
            types::RecordContent::A(a) => endpoints::dns::DnsContent::A { content: *a },
            types::RecordContent::Aaaa(aaaa) => endpoints::dns::DnsContent::AAAA { content: *aaaa },
            types::RecordContent::Txt(txt) => endpoints::dns::DnsContent::TXT {
                content: txt.to_string(),
            },
        }
    }
}

struct CloudflareProviderConfig {
    ttl: Option<u32>,
    proxied: Option<bool>,
}

pub struct CloudflareProvider {
    api: CloudflareWrapper,
    config: CloudflareProviderConfig,
}

impl Provider for CloudflareProvider {
    fn from_config(config: &crate::config::Config) -> Result<Box<dyn Provider>, ProviderError> {
        let api = CloudflareWrapper::try_new(&config.cloudflare_api_token)?;

        Ok(Box::new(CloudflareProvider {
            api,
            config: CloudflareProviderConfig {
                ttl: config.ttl,
                proxied: config.cloudflare_proxied,
            },
        }))
    }

    fn read_records(&self) -> Result<Vec<types::DnsRecord>, ProviderError> {
        info!("Reading zones from Cloudflare API");

        let zones = self.api.list_zones().map_err(|e| e.to_string())?.result;

        debug!("Found {} zones", zones.len());
        trace!("Collected zones {:?}", zones);

        let records = zones
            .iter()
            .map(|z| {
                self.api
                    .list_records(&z.id, &None, &None)
                    .map_err(|e| e.to_string())
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flat_map(|f| f.result)
            .filter_map(|r| types::DnsRecord::try_from(&r).ok())
            .collect::<Vec<types::DnsRecord>>();
        debug!("Read {} records from Cloudflare API", records.len());
        trace!("Collected Records: {:?}", records);
        Ok(records)
    }

    fn apply_plan(&self, plan: Plan) -> Vec<Result<(), ProviderError>> {
        let perform_action = |action: Action| -> Result<(), ProviderError> {
            match action {
                Action::Create(r) => {
                    debug!("Performing action CREATE for Record: {:?}", r);
                    self.api
                        .create_record(
                            &self
                                .api
                                .find_record_zone(&r)
                                .ok_or(format!("Could not find suitable zone for record {:?}", r))?
                                .id,
                            &r.name,
                            &self.config.ttl,
                            &self.config.proxied,
                            &r.content.into(),
                        )
                        .map_err(|e| e.to_string())
                        .map(|_| ())
                }
                Action::Update(r) => {
                    debug!("Performing action UPDATE for Record: {:?}", r);
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
                            &self.config.ttl,
                            &self.config.proxied,
                            &r.content.into(),
                        )
                        .map_err(|e| e.to_string())
                        .map(|_| ())
                }
                Action::Delete(r) => {
                    debug!("Performing action DELETE for Record: {:?}", r);
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
                        .map_err(|e| e.to_string())
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
