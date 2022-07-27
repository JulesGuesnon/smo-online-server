use std::{collections::HashMap, sync::Arc};

use crate::packet::Packet;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Default)]
pub struct Costume {
    pub body: String,
    pub cap: String,
}

pub struct Player {
    pub id: Uuid,
    pub costume: Option<Costume>,
    pub name: String,
    pub scenario: u8,
    pub is_2d: bool,
    pub is_speedrun: bool,
    pub is_seeking: bool,
    pub last_packet: Option<Packet>,
    pub shine_sync: Vec<u32>,
    pub loaded_save: bool,
}

// Player -> Player
// State related stuff -> Game state: Arc<RwLock<HashMap<Uuid, RwLock<State>>>>
impl Player {
    pub fn new(id: Uuid, name: String) -> Self {
        Self {
            id,
            costume: None,
            name,
            scenario: 0,
            is_2d: false,
            is_speedrun: false,
            is_seeking: false,
            last_packet: None,
            shine_sync: Vec::new(),
            loaded_save: false,
        }
    }
}

impl Player {
    pub fn set_costume(&mut self, body: String, cap: String) {
        self.costume = Some(Costume { body, cap });
    }
}

type SharedPlayer = Arc<RwLock<Player>>;
pub struct Players {
    players: RwLock<HashMap<Uuid, SharedPlayer>>,
}

impl Players {
    pub fn new() -> Self {
        Self {
            players: RwLock::default(),
        }
    }

    pub async fn get(&self, id: &Uuid) -> Option<SharedPlayer> {
        let players = self.players.read().await;

        players.get(id).map(|p| p.clone())
    }

    pub async fn remove(&self, id: &Uuid) -> Option<SharedPlayer> {
        let mut players = self.players.write().await;

        players.remove(id)
    }

    pub async fn add(&self, player: Player) -> SharedPlayer {
        let mut players = self.players.write().await;

        let id = player.id.clone();
        let player = Arc::new(RwLock::new(player));

        let player_ref = player.clone();

        players.insert(id, player);

        player_ref
    }
}
