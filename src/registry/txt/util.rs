use crate::{
    provider::{DnsRecord, RecordContent},
    registry::Domain,
};

use super::RegisteredDomain;

impl Domain {
    pub fn txt_match(&self, m: &str) -> Option<Vec<RecordContent>> {
        todo!()
    }
}

pub fn rec_into_reg_d(rec: &DnsRecord, reg_d: &mut RegisteredDomain) {
    match &rec.content {
        crate::provider::RecordContent::A(a) => {
            if !reg_d.domain.a.contains(a) {
                reg_d.domain.a.push(a.to_owned());
            }
        }
        crate::provider::RecordContent::Aaaa(aaaa) => {
            if !reg_d.domain.aaaa.contains(aaaa) {
                reg_d.domain.aaaa.push(aaaa.to_owned());
            }
        }
        crate::provider::RecordContent::Txt(txt) => {
            if !reg_d.domain.txt.contains(txt) {
                reg_d.domain.txt.push(txt.to_owned());
            }
        }
    }
}
