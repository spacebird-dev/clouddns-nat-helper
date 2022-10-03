use cloudflare::{endpoints, framework::response::ApiFailure};

use crate::{provider::ProviderError, types};

impl From<ApiFailure> for ProviderError {
    fn from(f: ApiFailure) -> Self {
        match f {
            ApiFailure::Error(s, errs) => ProviderError {
                msg: format!("[{}] {:?}", s, errs.errors),
            },
            ApiFailure::Invalid(_) => todo!(),
        }
    }
}

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
