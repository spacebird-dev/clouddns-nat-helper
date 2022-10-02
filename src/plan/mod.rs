use crate::types::DnsRecord;

pub struct Plan {
    pub actions: Vec<Action>,
}

pub enum Action {
    Create(DnsRecord),
    Update(DnsRecord),
    Delete(DnsRecord),
}
