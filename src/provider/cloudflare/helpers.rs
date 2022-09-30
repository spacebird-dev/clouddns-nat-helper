use cloudflare::{
    endpoints::zone::{ListZones, ListZonesParams},
    framework::endpoint::Endpoint,
};

pub trait CustomEndpoint: Endpoint + PaginatedEndpoint {}

pub trait PaginatedEndpoint {
    fn page(&self) -> Option<u32>;
    fn set_page(&mut self, page: u32);
    fn page_size(&self) -> u32;
    fn set_page_size(&mut self, size: u32);
}

impl PaginatedEndpoint for ListZones {
    fn page(&self) -> Option<u32> {
        self.params.page
    }

    fn set_page(&mut self, page: u32) {
        self.params.page = Some(page);
    }

    fn page_size(&self) -> u32 {
        self.params.per_page.unwrap_or(20)
    }

    fn set_page_size(&mut self, size: u32) {
        self.params.per_page = Some(size);
    }
}
