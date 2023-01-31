use cloudflare::{endpoints, framework::response::ApiFailure};

use crate::provider::{DnsRecord, ProviderError, RecordContent};

impl From<ApiFailure> for ProviderError {
    fn from(e: ApiFailure) -> Self {
        ProviderError::Internal(e.to_string())
    }
}

impl TryFrom<&endpoints::dns::DnsRecord> for DnsRecord {
    type Error = String;

    fn try_from(r: &endpoints::dns::DnsRecord) -> Result<Self, Self::Error> {
        let converted_content = match &r.content {
            endpoints::dns::DnsContent::A { content } => RecordContent::A(*content),
            endpoints::dns::DnsContent::AAAA { content } => RecordContent::Aaaa(*content),
            endpoints::dns::DnsContent::TXT { content } => RecordContent::Txt(content.to_owned()),
            _ => return Err(format!("Invalid record type: {:?}", r.content)),
        };
        Ok(DnsRecord {
            domain_name: r.name.to_owned(),
            content: converted_content,
        })
    }
}

impl From<RecordContent> for endpoints::dns::DnsContent {
    fn from(c: RecordContent) -> Self {
        match &c {
            RecordContent::A(a) => endpoints::dns::DnsContent::A { content: *a },
            RecordContent::Aaaa(aaaa) => endpoints::dns::DnsContent::AAAA { content: *aaaa },
            RecordContent::Txt(txt) => endpoints::dns::DnsContent::TXT {
                content: txt.to_owned(),
            },
        }
    }
}
