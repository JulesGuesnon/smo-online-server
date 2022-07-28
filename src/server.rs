use crate::{
    packet::{ConnectionType, Content, Header, Packet, TagUpdate, HEADER_SIZE},
    peer::Peer,
    players::{Player, Players, SharedPlayer},
    settings::Settings,
};
use anyhow::anyhow;
use anyhow::Result;
use bytes::Bytes;
use chrono::Duration;
use futures::{future::join_all, Future};
use glam::{Mat4, Quat, Vec3};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tokio::{
    fs::File,
    io::{split, AsyncReadExt, AsyncWriteExt, ReadHalf},
    net::TcpStream,
    sync::RwLock,
    time::sleep,
};
use tracing::{debug, info};
use uuid::Uuid;

const MAX_PLAYER: i16 = 10;

pub struct Server {
    peers: RwLock<HashMap<Uuid, Peer>>,
    shine_bag: RwLock<HashSet<i32>>,
    players: Players,
    settings: Settings,
}

impl Server {
    pub fn new(settings: Settings) -> Self {
        Self {
            peers: RwLock::default(),
            shine_bag: RwLock::default(),
            players: Players::new(),
            settings,
        }
    }

    async fn broadcast(&self, packet: Packet) {
        let peers = self.peers.read().await;

        join_all(
            peers
                .iter()
                .filter(|(_, p)| p.connected && p.id != packet.id)
                .map(|(_, p)| p.send(packet.clone())),
        )
        .await;
    }

    async fn broadcast_map<F, Fut>(&self, packet: Packet, map: F)
    where
        F: Fn(SharedPlayer, Packet) -> Fut,
        Fut: Future<Output = Packet>,
    {
        let peers = self.peers.read().await;

        join_all(
            peers
                .iter()
                .filter(|(_, p)| p.connected && p.id != packet.id)
                .map(|(_, peer)| async {
                    let packet = match self.players.get(&packet.id).await {
                        Some(p) => (map)(p, packet.clone()).await,
                        None => packet.clone(),
                    };

                    peer.send(packet).await;
                }),
        )
        .await;
    }

    pub async fn handle_connection(self: Arc<Self>, socket: TcpStream) -> Result<()> {
        let ip = socket.peer_addr()?;
        let (mut reader, writer) = split(socket);

        let mut peer = Peer::new(ip, writer);
        let id = peer.id.clone();

        peer.send(Packet::new(
            peer.id,
            Content::Init {
                max_player: MAX_PLAYER,
            },
        ))
        .await;

        let packet = receive_packet(&mut reader).await?;

        if !packet.content.is_connect() {
            debug!(
                "Player {} didn't send connection packet on first connection",
                packet.id
            );
            return Err(anyhow!("Didn't receive connection packet as first packet"));
        }

        let peers = self.peers.read().await;

        let connected_peers = peers
            .iter()
            .fold(0, |acc, p| if p.1.connected { acc + 1 } else { 0 });

        if connected_peers == MAX_PLAYER {
            info!("Player {} couldn't join server is full", packet.id);
            return Err(anyhow!("Server full"));
        }

        drop(peers);

        let mut peers = self.peers.write().await;

        // Remove stales clients and only keep the disconnected one
        let _ = peers.remove(&packet.id);

        match (packet.content, self.players.get(&packet.id).await) {
            // Player already exist so reconnecting
            (_, Some(_)) => {
                debug!("Client {} attempting to reconnect", id);

                peer.id = packet.id;
                peers.insert(packet.id, peer);
            }
            // Player doesn't exist so we create it
            (
                Content::Connect {
                    type_: _,
                    max_player: _,
                    client,
                },
                None,
            ) => {
                debug!("Client {} with id {} is joining", client, packet.id);
                peer.id = packet.id;

                let player = Player::new(packet.id, client);

                let _ = self.players.add(player).await;

                let peer = self.on_new_peer(peer).await?;

                peers.insert(packet.id, peer);
            }
            _ => {
                debug!("This case isn't supposed to be reach");
                return Err(anyhow!("This case isn't supposed to be reach"));
            }
        }

        let peers = self.peers.read().await;

        let peer = peers
            .get(&id)
            .ok_or(anyhow!("Player is supposed to be in the HashMap"))?;

        for (uuid, peer) in self.peers.read().await.iter() {
            if *uuid == id {
                continue;
            }

            let player = self
                .players
                .get(uuid)
                .await
                .expect("Peers and Players are desynchronized");

            let player = player.read().await;

            let _ = peer
                .send(Packet::new(
                    player.id,
                    Content::Connect {
                        type_: ConnectionType::First,
                        max_player: MAX_PLAYER as u16,
                        client: player.name.clone(),
                    },
                ))
                .await;

            if let Some(costume) = &player.costume {
                let _ = peer
                    .send(Packet::new(
                        player.id,
                        Content::Costume {
                            body: costume.body.clone(),
                            cap: costume.cap.clone(),
                        },
                    ))
                    .await;
            }

            drop(player);
        }

        drop(peer);
        drop(peers);

        let player = self
            .players
            .get(&id)
            .await
            .expect("Player is supposed to be here");

        loop {
            let packet = receive_packet(&mut reader).await?;

            if packet.id != id {
                debug!("Id mismatch: received {} - expecting {}", packet.id, id);

                return Err(anyhow!(
                    "Id mismatch: received {} - expecting {}",
                    packet.id,
                    id
                ));
            }

            let should_broadcast = match &packet.content {
                Content::Costume { body, cap } => {
                    let mut player = player.write().await;

                    player.set_costume(body.clone(), cap.clone());
                    player.loaded_save = true;

                    tokio::spawn({
                        let server = self.clone();
                        let id = player.id.clone();

                        async move {
                            let _ = server.sync_player_shine_bag(id).await;
                        }
                    });

                    true
                }
                Content::Game {
                    is_2d,
                    scenario,
                    stage,
                } => {
                    let mut player = player.write().await;

                    player.scenario = Some(*scenario);
                    player.is_2d = *is_2d;
                    player.last_game_packet = Some(packet.clone());

                    if stage == "CapWorldHomeStage" && *scenario == 0 {
                        player.is_speedrun = true;
                        player.shine_sync.clear();
                        let mut shine_bag = self.shine_bag.write().await;

                        shine_bag.clear();

                        self.persist_shines().await;

                        info!("Entered Cap on new save, preventing moon sync until Cascade");
                    } else if stage == "WaterfallWorldHomeStage" {
                        let was_speedrun = player.is_speedrun;
                        player.is_speedrun = false;

                        if was_speedrun {
                            let id = player.id.clone();

                            tokio::spawn({
                                let server = self.clone();
                                async move {
                                    info!(
                                    "Entered Cascade with moon sync disabled, enabling moon sync"
                                );
                                    sleep(std::time::Duration::from_secs(15)).await;
                                    let _ = server.sync_player_shine_bag(id).await;
                                }
                            });
                        }
                    }

                    if self.settings.is_merge_enabled {
                        self.broadcast_map(packet.clone(), |player, packet| async move {
                            match packet.content {
                                Content::Game {
                                    is_2d,
                                    scenario: _,
                                    stage,
                                } => {
                                    let player = player.read().await;

                                    let scenario = player.scenario.unwrap_or(200);
                                    Packet::new(
                                        packet.id,
                                        Content::Game {
                                            is_2d,
                                            scenario,
                                            stage,
                                        },
                                    )
                                }
                                _ => packet,
                            }
                        })
                        .await;

                        false
                    } else {
                        true
                    }
                }
                Content::Tag {
                    update_type: TagUpdate::State,
                    is_it,
                    seconds: _,
                    minutes: _,
                } => {
                    let mut player = player.write().await;

                    player.is_seeking = *is_it;

                    true
                }
                Content::Tag {
                    update_type: TagUpdate::Time,
                    is_it: _,
                    seconds,
                    minutes,
                } => {
                    let mut player = player.write().await;

                    player.time =
                        Duration::minutes(*minutes as i64) + Duration::seconds(*seconds as i64);

                    true
                }
                Content::Shine { id } => {
                    let mut player = player.write().await;

                    if player.loaded_save {
                        let mut shine_bag = self.shine_bag.write().await;

                        shine_bag.insert(id.clone());
                        if player.shine_sync.get(id).is_none() {
                            info!("Got moon {}", id);
                            player.shine_sync.insert(id.clone());

                            tokio::spawn({
                                let server = self.clone();
                                async move {
                                    server.sync_shine_bag().await;
                                }
                            });
                        }
                    }

                    true
                }
                Content::Player {
                    position,
                    quaternion,
                    animation_blend_weights,
                    act,
                    subact,
                } if self.settings.flip_in(&packet.id) => {
                    let size = player.read().await.size();

                    tokio::spawn({
                        let server = self.clone();

                        let id = packet.id.clone();
                        let position = position.clone();
                        let quaternion = quaternion.clone();
                        let animation_blend_weights = animation_blend_weights.clone();
                        let act = act.clone();
                        let subact = subact.clone();

                        let position = position + Vec3::Y * size;
                        let quaternion = quaternion
                            * Quat::from_mat4(&Mat4::from_rotation_x(std::f32::consts::PI))
                            * Quat::from_mat4(&Mat4::from_rotation_y(std::f32::consts::PI));

                        async move {
                            server
                                .broadcast(Packet::new(
                                    id,
                                    Content::Player {
                                        position,
                                        quaternion,
                                        animation_blend_weights,
                                        act,
                                        subact,
                                    },
                                ))
                                .await;
                        }
                    });

                    false
                }
                Content::Player {
                    position,
                    quaternion,
                    animation_blend_weights,
                    act,
                    subact,
                } if self.settings.flip_not_in(&packet.id) => {
                    let size = player.read().await.size();

                    tokio::spawn({
                        let server = self.clone();

                        let id = packet.id.clone();
                        let position = position.clone();
                        let quaternion = quaternion.clone();
                        let animation_blend_weights = animation_blend_weights.clone();
                        let act = act.clone();
                        let subact = subact.clone();

                        async move {
                            server
                                .broadcast_map(packet.clone(), |player, packet| async {
                                    let position = position + Vec3::Y * size;
                                    let quaternion = quaternion
                                        * Quat::from_mat4(&Mat4::from_rotation_x(
                                            std::f32::consts::PI,
                                        ))
                                        * Quat::from_mat4(&Mat4::from_rotation_y(
                                            std::f32::consts::PI,
                                        ));

                                    Packet::new(
                                        id,
                                        Content::Player {
                                            position,
                                            quaternion,
                                            animation_blend_weights: animation_blend_weights
                                                .clone(),
                                            act: act.clone(),
                                            subact: subact.clone(),
                                        },
                                    )
                                })
                                .await
                        }
                    });

                    false
                }
                Content::Disconnect => break,
                _ => true,
            };

            self.broadcast(packet).await;
        }

        // TODO: Find out when peers & players are cleaned
        let mut peers = self.peers.write().await;
        let mut peer = peers.get_mut(&id).expect("Peer is supposed to be here");

        peer.connected = false;
        peer.disconnect().await;

        Ok(())
    }

    async fn on_new_peer(&self, peer: Peer) -> Result<Peer> {
        let is_ip_banned = self
            .settings
            .ban_list
            .ips
            .iter()
            .find(|addr| **addr == peer.ip)
            .is_some();

        let is_id_banned = self
            .settings
            .ban_list
            .ids
            .iter()
            .find(|addr| **addr == peer.id)
            .is_some();

        if is_id_banned || is_ip_banned {
            info!(
                "Banned player {} with ip {} tried to joined",
                peer.ip, peer.id
            );

            Err(anyhow!(
                "Banned player {} with ip {} tried to joined",
                peer.ip,
                peer.id
            ))
        } else {
            let packets = self.players.get_last_game_packets().await;

            for packet in packets {
                peer.send(packet).await;
            }

            Ok(peer)
        }
    }

    async fn sync_player_shine_bag(&self, id: Uuid) -> Result<()> {
        let player = self
            .players
            .get(&id)
            .await
            .ok_or(anyhow!("Couldn't find player"))?;

        let mut player = player.write().await;

        if player.is_speedrun {
            return Err(anyhow!("Player is in speedrun mode"));
        }

        let bag = self.shine_bag.read().await;
        let peers = self.peers.read().await;
        let peer = peers.get(&id).ok_or(anyhow!("Couldn't find peer"))?;

        for shine in bag.difference(&player.shine_sync.clone()) {
            player.shine_sync.insert(shine.clone());
            peer.send(Packet::new(
                id.clone(),
                Content::Shine { id: shine.clone() },
            ))
            .await
        }

        Ok(())
    }

    async fn persist_shines(&self) {
        if !self.settings.persist_shines.enabled {
            return;
        }

        let shines = self.shine_bag.read().await;

        let shines = shines.clone();
        let file_name = self.settings.persist_shines.file_name.clone();

        tokio::spawn(async move {
            let serialized = serde_json::to_string(&shines).unwrap();

            let mut file = File::open(file_name)
                .await
                .expect("Shine file can't be opened");

            let _ = file.write_all(serialized.as_bytes()).await;
        });
    }

    async fn sync_shine_bag(&self) {
        self.persist_shines().await;
        join_all(
            self.players
                .all_ids()
                .await
                .into_iter()
                .map(|id| self.sync_player_shine_bag(id)),
        )
        .await;
    }

    pub async fn load_shines(&self) {
        if !self.settings.persist_shines.enabled {
            return;
        }

        let mut file = File::open(&self.settings.persist_shines.file_name)
            .await
            .expect("Shine file can't be opened");

        let mut content = String::from("");
        let _ = file.read_to_string(&mut content).await;

        let deserialized = serde_json::from_str(&content).unwrap();

        let mut shines = self.shine_bag.write().await;

        *shines = deserialized;
    }
}

async fn receive_packet(reader: &mut ReadHalf<TcpStream>) -> Result<Packet> {
    let mut header_buf = [0; HEADER_SIZE];

    match reader.read_exact(&mut header_buf).await {
        Ok(n) if n == 0 => return Ok(Packet::new(Uuid::nil(), Content::Disconnect)),
        Ok(_) => (),
        Err(e) => {
            debug!("Error reading header {}", e);
            return Err(anyhow!(e));
        }
    };

    let header = match Header::from_bytes(Bytes::from(header_buf.to_vec())) {
        Ok(h) => h,
        Err(e) => {
            return Err(e);
        }
    };

    let body = if header.packet_size > 0 {
        let mut body_buf = vec![0; header.packet_size];

        match reader.read_exact(&mut body_buf).await {
            Ok(n) if n == 0 => return Err(anyhow!("End of file reached")),
            Ok(_) => (),
            Err(e) => {
                debug!("Error reading header {}", e);
                return Err(anyhow!(e));
            }
        };

        Bytes::from(body_buf)
    } else {
        Bytes::new()
    };

    Ok(header.make_packet(body)?)
}
