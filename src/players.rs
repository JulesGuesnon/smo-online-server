use crate::packet::Packet;
use chrono::Duration;
use futures::future::join_all;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
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
    pub scenario: Option<u8>,
    pub is_2d: bool,
    pub is_speedrun: bool,
    pub is_seeking: bool,
    pub last_game_packet: Option<Packet>,
    pub shine_sync: HashSet<i32>,
    pub loaded_save: bool,
    pub time: Duration,
}

// Player -> Player
// State related stuff -> Game state: Arc<RwLock<HashMap<Uuid, RwLock<State>>>>
impl Player {
    pub fn new(id: Uuid, name: String) -> Self {
        Self {
            id,
            costume: None,
            name,
            scenario: None,
            is_2d: false,
            is_speedrun: false,
            is_seeking: false,
            last_game_packet: None,
            shine_sync: HashSet::new(),
            loaded_save: false,
            time: Duration::zero(),
        }
    }
}

impl Player {
    pub fn set_costume(&mut self, body: String, cap: String) {
        self.costume = Some(Costume { body, cap });
    }
}

pub type SharedPlayer = Arc<RwLock<Player>>;
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

    pub async fn all_ids(&self) -> Vec<Uuid> {
        let players = self.players.read().await;

        players.keys().map(|k| k.clone()).collect()
    }

    pub async fn remove(&self, id: &Uuid) -> Option<SharedPlayer> {
        let mut players = self.players.write().await;

        players.remove(id)
    }

    pub async fn get_last_game_packets(&self) -> Vec<Packet> {
        let players = self.players.read().await;

        let players = join_all(players.iter().map(|(_, p)| p.read())).await;

        players
            .iter()
            .filter_map(|p| p.last_game_packet.clone())
            .collect()
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
