use crate::{provider::DnsRecord, registry::Domain};

pub const TXT_RECORD_IDENT: &str = "clouddns_nat";
pub const TXT_RECORD_SEP: &str = ";";
// Returns the TXT ownership record content for a given tenant
// Global function as we need to call it in new() before we can create our TxtRegistry
pub fn txt_record_string(tenant: &str) -> String {
    format!("{}_{}{}rec: A", TXT_RECORD_IDENT, tenant, TXT_RECORD_SEP)
}

pub fn rec_into_d(rec: &DnsRecord, d: &mut Domain) {
    match &rec.content {
        crate::provider::RecordContent::A(a) => {
            if !d.a.contains(a) {
                d.a.push(a.to_owned());
            }
        }
        crate::provider::RecordContent::Aaaa(aaaa) => {
            if !d.aaaa.contains(aaaa) {
                d.aaaa.push(aaaa.to_owned());
            }
        }
        crate::provider::RecordContent::Txt(txt) => {
            if !d.txt.contains(txt) {
                d.txt.push(txt.to_owned());
            }
        }
    }
}
