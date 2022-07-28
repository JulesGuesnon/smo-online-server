use serde::{Deserialize, Serialize};
use std::{net::IpAddr, str::FromStr};
use tokio::{
    fs::OpenOptions,
    io::{AsyncReadExt, AsyncWriteExt},
};
use tracing::info;
use uuid::Uuid;

#[derive(PartialEq, Eq, Deserialize, Serialize)]
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

#[derive(Default, Deserialize, Serialize)]
pub struct Flip {
    pub enabled: bool,
    pub players: Vec<Uuid>,
    pub pov: FlipPov,
}

#[derive(Deserialize, Serialize)]
pub struct BanList {
    pub enabled: bool,
    pub ids: Vec<Uuid>,
    pub ips: Vec<IpAddr>,
}

impl Default for BanList {
    fn default() -> Self {
        Self {
            enabled: false,
            ids: vec![],
            ips: vec![],
        }
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

#[derive(Deserialize, Serialize)]
pub struct Scenario {
    pub merge_enabled: bool,
}

impl Default for Scenario {
    fn default() -> Self {
        Self {
            merge_enabled: false,
        }
    }
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
}

impl Settings {
    pub async fn load() -> Self {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open("settings.json")
            .await
            .expect("Settings couldn't be loaded or created");

        let mut content = String::from("");
        file.read_to_string(&mut content).await.unwrap();

        match serde_json::from_str(&content) {
            Ok(v) => {
                info!("Loaded settings.json");
                v
            }
            Err(_) => {
                info!("Creating file settings.json. If you want to update it, stop the server, modify the file and restart the server");

                let settings = Self::default();

                let serialized = serde_json::to_string_pretty(&settings).unwrap();

                file.write_all(serialized.as_bytes()).await.unwrap();

                settings
            }
        }
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
}
