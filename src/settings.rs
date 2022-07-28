use std::net::SocketAddr;

use uuid::Uuid;

#[derive(PartialEq, Eq)]
pub enum FlipPov {
    Both,
    Self_,
    Others,
}

impl Default for FlipPov {
    fn default() -> Self {
        Self::Both
    }
}

#[derive(Default)]
pub struct Flip {
    pub enabled: bool,
    pub players: Vec<Uuid>,
    pub pov: FlipPov,
}

#[derive(Default)]
pub struct Instance {
    pub flip: Flip,
}

#[derive(Default)]
pub struct BanList {
    pub ids: Vec<Uuid>,
    pub ips: Vec<SocketAddr>,
}

#[derive(Default)]
pub struct PersistShines {
    pub enabled: bool,
    pub file_name: String,
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
    pub persist_shines: PersistShines,
    pub instance: Instance,
}

impl Settings {
    pub fn flip_in(&self, id: &Uuid) -> bool {
        self.instance.flip.enabled
            && (self.instance.flip.pov == FlipPov::Both
                || self.instance.flip.pov == FlipPov::Others)
            && self.instance.flip.players.contains(id)
    }

    pub fn flip_not_in(&self, id: &Uuid) -> bool {
        self.instance.flip.enabled
            && (self.instance.flip.pov == FlipPov::Both || self.instance.flip.pov == FlipPov::Self_)
            && !self.instance.flip.players.contains(id)
    }
}
