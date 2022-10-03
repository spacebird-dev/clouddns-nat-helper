use crate::types::DnsRecord;

#[derive(Debug)]
pub struct Plan {
    pub actions: Vec<Action>,
}

impl Plan {
    fn new() -> Plan {
        todo!()
    }
}

#[derive(Debug)]
pub enum Action {
    Create(DnsRecord),
    Update(DnsRecord),
    Delete(DnsRecord),
}
