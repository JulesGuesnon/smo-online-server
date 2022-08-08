use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use chrono::Duration;
use futures::future::join_all;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::packet::{Content, Packet};

const MARIO_SIZE: f32 = 160.;
const MARIO_SIZE_2D: f32 = 180.;

#[derive(Debug, Default)]
pub struct Costume {
    pub body: String,
    pub cap: String,
}

#[derive(Debug)]
pub struct Player {
    pub id: Uuid,
    pub costume: Option<Costume>,
    pub name: String,
    pub scenario: Option<u8>,
    pub is_2d: bool,
    pub is_speedrun: bool,
    pub is_seeking: bool,
    pub last_game_packet: Option<Packet>,
    pub last_position: Option<Content>,
    // id, is_grand
    pub shine_sync: HashSet<i32>,
    pub loaded_save: bool,
    pub time: Duration,
}

impl Default for Player {
    fn default() -> Self {
        Self {
            id: Default::default(),
            costume: Default::default(),
            name: Default::default(),
            scenario: Default::default(),
            is_2d: Default::default(),
            is_speedrun: Default::default(),
            is_seeking: Default::default(),
            last_game_packet: Default::default(),
            last_position: Default::default(),
            shine_sync: Default::default(),
            loaded_save: Default::default(),
            time: Duration::seconds(0),
        }
    }
}

impl Player {
    #[inline]
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
            last_position: None,
            shine_sync: HashSet::new(),
            loaded_save: false,
            time: Duration::zero(),
        }
    }
}

impl Player {
    #[inline]
    pub fn set_costume(&mut self, body: String, cap: String) {
        self.costume = Some(Costume { body, cap });
    }

    #[inline(always)]
    pub fn size(&self) -> f32 {
        if self.is_2d {
            MARIO_SIZE_2D
        } else {
            MARIO_SIZE
        }
    }

    pub fn get_stage(&self) -> Option<String> {
        self.last_game_packet
            .as_ref()
            .and_then(|packet| match &packet.content {
                Content::Game {
                    is_2d: _,
                    scenario: _,
                    stage,
                } => Some(stage.clone()),
                _ => None,
            })
    }
}

pub type SharedPlayer = Arc<RwLock<Player>>;
pub struct Players {
    players: RwLock<HashMap<Uuid, SharedPlayer>>,
    names: RwLock<HashMap<Uuid, String>>,
}

impl Players {
    #[inline]
    pub fn new() -> Self {
        Self {
            players: RwLock::default(),
            names: RwLock::default(),
        }
    }

    pub async fn get(&self, id: &Uuid) -> Option<SharedPlayer> {
        let players = self.players.read().await;

        players.get(id).cloned()
    }

    pub async fn all(&self) -> Vec<SharedPlayer> {
        let players = self.players.read().await;

        players.values().cloned().collect()
    }

    pub async fn all_from_ids(&self, ids: Vec<Uuid>) -> Vec<SharedPlayer> {
        let players = self.players.read().await;

        ids.iter()
            .filter_map(|id| players.get(id).cloned())
            .collect()
    }

    pub async fn all_ids(&self) -> Vec<Uuid> {
        let players = self.players.read().await;

        players.keys().copied().collect()
    }

    pub async fn all_ids_and_names(&self) -> Vec<(Uuid, String)> {
        let players = self.players.read().await;

        let players = join_all(
            players
                .iter()
                .map(|(id, p)| async { (*id, p.read().await) }),
        )
        .await;

        players
            .into_iter()
            .map(|(id, player)| (id, player.name.clone()))
            .collect()
    }

    pub async fn get_id_by_name(&self, username: String) -> Option<Uuid> {
        let names = self.names.read().await;

        names
            .iter()
            .find(|(_, name)| name.to_lowercase() == username.to_lowercase())
            .map(|(id, _)| *id)
    }

    // No idea when to remove a player for now
    // pub async fn remove(&self, id: &Uuid) -> Option<SharedPlayer> {
    //     let mut players = self.players.write().await;

    //     players.remove(id)
    // }

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
        let mut names = self.names.write().await;

        let id = player.id;

        names.insert(id, player.name.clone());

        let player = Arc::new(RwLock::new(player));

        let player_ref = player.clone();

        players.insert(id, player);

        player_ref
    }
}
