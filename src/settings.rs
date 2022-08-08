use std::net::IpAddr;
use std::path::PathBuf;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

#[derive(Debug, PartialEq, Eq, Deserialize, Serialize, Clone)]
pub enum FlipPov {
    Both,
    Self_,
    Others,
}

impl FlipPov {
    pub fn from_str(string: &str) -> Result<Self, String> {
        match string.to_lowercase().as_str() {
            "both" => Ok(Self::Both),
            "self" => Ok(Self::Self_),
            "others" => Ok(Self::Others),
            v => Err(format!(
                "Invalid value {}, expected both or self or others",
                v
            )),
        }
    }

    #[inline]
    pub fn to_str(&self) -> &'static str {
        match self {
            Self::Both => "both",
            Self::Others => "others",
            Self::Self_ => "self",
        }
    }
}

impl Default for FlipPov {
    #[inline(always)]
    fn default() -> Self {
        Self::Both
    }
}

#[derive(Default, Deserialize, Serialize)]
pub struct Flip {
    pub enabled: bool,
    pub players: Vec<Uuid>,
    pub pov: FlipPov,
}

#[derive(Deserialize, Serialize)]
pub struct SpecialCostumes {
    pub costumes: Vec<String>,
    pub allowed_players: Vec<Uuid>,
}

impl Default for SpecialCostumes {
    fn default() -> Self {
        Self {
            costumes: vec!["MarioInvisible".to_owned()],
            allowed_players: Default::default(),
        }
    }
}

#[derive(Default, Deserialize, Serialize)]
pub struct BanList {
    pub enabled: bool,
    pub ids: Vec<Uuid>,
    pub ips: Vec<IpAddr>,
}

impl BanList {
    pub fn ban(&mut self, id: Uuid, ip: Option<IpAddr>) {
        self.ids.push(id);

        if let Some(ip) = ip {
            self.ips.push(ip);
        }
    }

    pub fn is_ip_ban(&self, ip: &IpAddr) -> bool {
        self.ips.contains(ip)
    }
}

#[derive(Deserialize, Serialize)]
pub struct PersistShines {
    pub enabled: bool,
    pub file_name: String,
}

impl Default for PersistShines {
    fn default() -> Self {
        Self {
            enabled: false,
            file_name: String::from("./moons.json"),
        }
    }
}

#[derive(Default, Deserialize, Serialize)]
pub struct Scenario {
    pub merge_enabled: bool,
}

#[derive(Deserialize, Serialize)]
pub struct Server {
    pub address: IpAddr,
    pub port: u32,
    pub max_players: i16,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            address: IpAddr::from_str("0.0.0.0").unwrap(),
            port: 1027,
            max_players: 8,
        }
    }
}

#[derive(Default, Deserialize, Serialize)]
pub struct Settings {
    pub server: Server,
    pub ban_list: BanList,
    pub scenario: Scenario,
    pub persist_shines: PersistShines,
    pub flip: Flip,
    pub special_costumes: SpecialCostumes,
}

impl Settings {
    #[inline(always)]
    fn path_buf() -> PathBuf {
        PathBuf::from("./settings.json")
    }

    pub async fn load() -> Self {
        let path = Self::path_buf();
        if !path.exists() {
            return Self::load_default().await;
        }

        let body = tokio::fs::read(path)
            .await
            .expect("Failed to read settings");

        match serde_json::from_slice(&body) {
            Ok(v) => {
                info!("Loaded settings.json");
                v
            }
            Err(_) => {
                info!("Creating file settings.json. If you want to update it, stop the server, modify the file and restart the server");
                Self::load_default().await
            }
        }
    }

    async fn load_default() -> Self {
        let settings = Self::default();
        settings.save().await;

        settings
    }

    pub async fn save(&self) {
        let path = Self::path_buf();
        let serialized = serde_json::to_string_pretty(self).unwrap();

        tokio::fs::write(path, serialized)
            .await
            .expect("Settings failed to save");
    }

    pub fn flip_in(&self, id: &Uuid) -> bool {
        self.flip.enabled
            && (self.flip.pov == FlipPov::Both || self.flip.pov == FlipPov::Others)
            && self.flip.players.contains(id)
    }

    pub fn flip_not_in(&self, id: &Uuid) -> bool {
        self.flip.enabled
            && (self.flip.pov == FlipPov::Both || self.flip.pov == FlipPov::Self_)
            && !self.flip.players.contains(id)
    }

    pub fn is_special_costume(&self, costume: &String) -> bool {
        self.special_costumes.costumes.contains(costume)
    }

    pub fn special_costume_allowed(&self, id: &Uuid) -> bool {
        self.special_costumes.allowed_players.contains(id)
    }
}
