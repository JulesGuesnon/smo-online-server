use std::net::SocketAddr;

use uuid::Uuid;

#[derive(Default)]
pub struct BanList {
    pub ids: Vec<Uuid>,
    pub ips: Vec<SocketAddr>,
}

impl BanList {
    pub fn new(ids: Vec<Uuid>, ips: Vec<SocketAddr>) -> Self {
        Self { ids, ips }
    }
}

#[derive(Default)]
pub struct Settings {
    pub ban_list: BanList,
    pub is_merge_enabled: bool,
}
