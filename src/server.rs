use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use bytes::Bytes;
use chrono::Duration;
use color_eyre::eyre::eyre;
use color_eyre::Result;
use futures::future::join_all;
use futures::Future;
use glam::{Mat4, Quat, Vec3};
use tokio::fs::OpenOptions;
use tokio::io::{split, AsyncReadExt, ReadHalf};
use tokio::net::TcpStream;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::{debug, info};
use uuid::Uuid;

use crate::packet::{ConnectionType, Content, Header, Packet, TagUpdate, HEADER_SIZE};
use crate::peer::Peer;
use crate::players::{Player, Players, SharedPlayer};
use crate::settings::Settings;

pub struct Server {
    pub peers: RwLock<HashMap<Uuid, Peer>>,
    pub shine_bag: RwLock<HashSet<i32>>,
    pub players: Players,
    pub settings: RwLock<Settings>,
}

impl Server {
    pub fn new(settings: Settings) -> Self {
        Self {
            peers: RwLock::default(),
            shine_bag: RwLock::default(),
            players: Players::new(),
            settings: RwLock::new(settings),
        }
    }

    pub async fn broadcast(&self, packet: Packet) {
        let peers = self.peers.read().await;

        join_all(
            peers
                .iter()
                .filter(|(_, p)| p.connected && p.id != packet.id)
                .map(|(_, p)| p.send(packet.clone())),
        )
        .await;
    }

    pub async fn broadcast_map<F, Fut>(&self, packet: Packet, map: F)
    where
        F: Fn(SharedPlayer, Packet) -> Fut,
        Fut: Future<Output = Option<Packet>>,
    {
        let peers = self.peers.read().await;

        join_all(
            peers
                .iter()
                .filter(|(_, p)| p.connected && p.id != packet.id)
                .map(|(_, peer)| async {
                    let packet = match self.players.get(&peer.id).await {
                        Some(p) => (map)(p, packet.clone()).await,
                        None => Some(packet.clone()),
                    };

                    if let Some(packet) = packet {
                        peer.send(packet).await;
                    }
                }),
        )
        .await;
    }

    pub async fn send_to(&self, id: &Uuid, packet: Packet) -> Result<()> {
        let peers = self.peers.read().await;

        if let Some(peer) = peers.get(id) {
            peer.send(packet).await;

            Ok(())
        } else {
            Err(eyre!("User {} not found", id))
        }
    }

    pub async fn connected_peers(&self) -> Vec<Uuid> {
        let peers = self.peers.read().await;

        peers
            .iter()
            .filter_map(|(id, p)| if p.connected { Some(*id) } else { None })
            .collect()
    }

    pub async fn handle_connection(self: Arc<Self>, socket: TcpStream) -> Result<()> {
        let mut id = Uuid::nil();

        let run = || async {
            let ip = socket.peer_addr()?.ip();
            debug!(%ip, "Accepted incoming connection");

            let (mut reader, writer) = split(socket);

            let mut peer = Peer::new(ip, writer);

            peer.send(Packet::new(
                peer.id,
                Content::Init {
                    max_player: self.settings.read().await.server.max_players,
                },
            ))
            .await;

            let connect_packet = receive_packet(&mut reader).await?;

            if !connect_packet.content.is_connect() {
                debug!(
                    "Player {} didn't send connection packet on first connection",
                    connect_packet.id
                );
                return Err(eyre!("Didn't receive connection packet as first packet"));
            }

            let peers = self.peers.read().await;

            let connected_peers = peers
                .iter()
                .fold(0, |acc, p| if p.1.connected { acc + 1 } else { 0 });

            if connected_peers == self.settings.read().await.server.max_players {
                info!("Player {} couldn't join: server is full", connect_packet.id);
                return Err(eyre!("Server full"));
            }

            drop(peers);

            let mut peers = self.peers.write().await;

            // Remove stales clients and only keep the disconnected one
            if let Some(peer) = peers.remove(&connect_packet.id) {
                peer.disconnect().await;
            }

            let content = connect_packet.content.clone();
            match (content, self.players.get(&connect_packet.id).await) {
                // Player already exist so reconnecting
                (_, Some(player)) => {
                    let player = player.read().await;

                    peer.id = connect_packet.id;

                    let peer = self.on_new_peer(peer).await?;

                    id = connect_packet.id;
                    peers.insert(connect_packet.id, peer);
                    info!("[{}] {} reconnected", player.name, id);
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
                    info!("{} with id {} joining", client, connect_packet.id);
                    peer.id = connect_packet.id;
                    id = connect_packet.id;

                    let _ = self
                        .players
                        .add(Player::new(connect_packet.id, client))
                        .await;

                    let peer = self.on_new_peer(peer).await?;

                    peers.insert(connect_packet.id, peer);
                }
                _ => {
                    debug!("This case isn't supposed to be reach");
                    return Err(eyre!("This case isn't supposed to be reach"));
                }
            }

            tokio::spawn({
                let server = self.clone();

                async move {
                    server.broadcast(connect_packet).await;
                }
            });

            drop(peers);

            let peers = self.peers.read().await;

            let peer = peers
                .get(&id)
                .ok_or_else(|| eyre!("Peer is supposed to be in the HashMap"))?;

            for (uuid, other_peer) in self.peers.read().await.iter() {
                if *uuid == id || !other_peer.connected {
                    continue;
                }

                let player = self
                    .players
                    .get(uuid)
                    .await
                    .expect("Peers and Players are desynchronized");

                let player = player.read().await;

                peer.send(Packet::new(
                    player.id,
                    Content::Connect {
                        type_: ConnectionType::First,
                        max_player: self.settings.read().await.server.max_players as u16,
                        client: player.name.clone(),
                    },
                ))
                .await;

                if let Some(costume) = &player.costume {
                    peer.send(Packet::new(
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

            drop(peers);

            let player = self
                .players
                .get(&id)
                .await
                .expect("Player is supposed to be here");

            loop {
                let packet = receive_packet(&mut reader).await?;

                if packet.content.is_disconnect() {
                    break;
                } else if packet.id != id {
                    debug!("Id mismatch: received {} - expecting {}", packet.id, id);

                    return Err(eyre!(
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
                            let id = player.id;

                            async move {
                                let _ = server.sync_player_shine_bag(id).await;
                            }
                        });

                        let settings = self.settings.read().await;

                        let is_allowed_special = settings.special_costume_allowed(&player.id);
                        let is_cap_special = settings.is_special_costume(cap);
                        let is_body_special = settings.is_special_costume(body);

                        drop(settings);
                        let fallback = "Mario".to_owned();

                        let cap = match (is_cap_special, is_allowed_special) {
                            (true, false) => fallback.clone(),
                            _ => cap.clone(),
                        };

                        let body = match (is_body_special, is_allowed_special) {
                            (true, false) => fallback,
                            _ => body.clone(),
                        };

                        let outgoing = Packet {
                            id: packet.id,
                            content: Content::Costume { body, cap },
                        };

                        self.broadcast(outgoing).await;

                        false
                    }
                    Content::Game {
                        is_2d,
                        scenario,
                        stage: self_stage,
                    } => {
                        let mut player = player.write().await;
                        info!("{}: {}->{}", player.name, self_stage, scenario);

                        player.scenario = Some(*scenario);
                        player.is_2d = *is_2d;
                        player.last_game_packet = Some(packet.clone());

                        if self_stage == "CapWorldHomeStage" && *scenario == 0 {
                            player.is_speedrun = true;
                            player.shine_sync.clear();
                            let mut shine_bag = self.shine_bag.write().await;

                            shine_bag.clear();

                            tokio::spawn({
                                let server = self.clone();

                                async move {
                                    server.persist_shines().await;
                                }
                            });

                            info!("Entered Cap on new save, preventing moon sync until Cascade");
                        } else if self_stage == "WaterfallWorldHomeStage" {
                            let was_speedrun = player.is_speedrun;
                            player.is_speedrun = false;

                            if was_speedrun {
                                let id = player.id;

                                tokio::spawn({
                                    let server = self.clone();
                                    async move {
                                        info!("Entered Cascade with moon sync disabled, enabling moon sync");
                                        sleep(std::time::Duration::from_secs(15)).await;
                                        let _ = server.sync_player_shine_bag(id).await;
                                    }
                                });
                            }
                        }

                        drop(player);

                        let should_broadcast = if self.settings.read().await.scenario.merge_enabled
                        {
                            tokio::spawn({
                                let server = self.clone();
                                let packet = packet.clone();

                                async move {
                                    server
                                        .broadcast_map(packet, |player, packet| async move {
                                            let packet = match packet.content {
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
                                            };

                                            Some(packet)
                                        })
                                        .await;
                                }
                            });

                            false
                        } else {
                            true
                        };

                        // Send the position of all players when a player join a stage
                        // If we don't do so, people are gonna be invisible or to their previous position until they move
                        let peers = self.peers.read().await;
                        let peer = peers.get(&id);

                        if let Some(peer) = peer {
                            let players = self.players.all().await;

                            let positions = join_all(players.iter().map(|p| async move {
                                let player = p.read().await;

                                (player.get_stage(), player.id, player.last_position.clone())
                            }))
                            .await;

                            for (stage, id, position) in positions {
                                match (stage, &position) {
                                    (
                                        Some(player_stage),
                                        Some(Content::Player {
                                            position: _,
                                            quaternion: _,
                                            animation_blend_weights: _,
                                            act: _,
                                            subact: _,
                                        }),
                                    ) if &player_stage == self_stage => {
                                        peer.send(Packet::new(id, position.unwrap())).await
                                    }
                                    _ => (),
                                }
                            }
                        }

                        drop(peers);

                        should_broadcast
                    }
                    Content::Tag {
                        update_type,
                        is_it,
                        seconds,
                        minutes,
                    } => {
                        let mut player = player.write().await;

                        info!(
                            "{} is now {}",
                            player.name,
                            if *is_it { "seeker" } else { "hider" }
                        );

                        if (update_type & TagUpdate::State.as_byte()) != 0 {
                            player.is_seeking = *is_it;
                        }

                        if (update_type & TagUpdate::Time.as_byte()) != 0 {
                            player.time = Duration::minutes(i64::from(*minutes))
                                + Duration::seconds(i64::from(*seconds));
                        }

                        true
                    }
                    Content::Shine { id } => {
                        let mut player = player.write().await;

                        if player.loaded_save {
                            let mut shine_bag = self.shine_bag.write().await;

                            let shine = *id;

                            shine_bag.insert(shine);

                            if player.shine_sync.get(&shine).is_none() {
                                info!("Got moon {}", id);
                                player.shine_sync.insert(shine);

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
                        position: game_pos,
                        quaternion,
                        animation_blend_weights,
                        act,
                        subact,
                    } if self.settings.read().await.flip_in(&packet.id) => {
                        let mut player = player.write().await;
                        player.last_position = Some(packet.content.clone());
                        player.loaded_save = true;
                        let size = player.size();
                        let sender_stage = player.get_stage();

                        drop(player);

                        tokio::spawn({
                            let server = self.clone();

                            let id = packet.id;
                            let position = *game_pos;
                            let quaternion = *quaternion;
                            let animation_blend_weights = animation_blend_weights.clone();
                            let act = *act;
                            let subact = *subact;

                            let position = position + Vec3::Y * size;
                            let quaternion = quaternion
                                * Quat::from_mat4(&Mat4::from_rotation_x(std::f32::consts::PI))
                                * Quat::from_mat4(&Mat4::from_rotation_y(std::f32::consts::PI));

                            async move {
                                let packet = Packet::new(
                                    id,
                                    Content::Player {
                                        position,
                                        quaternion,
                                        animation_blend_weights,
                                        act,
                                        subact,
                                    },
                                );

                                server
                                    .broadcast_map(packet.clone(), |player, packet| {
                                        let sender_stage = sender_stage.clone();

                                        async move {
                                            let player = player.read().await;

                                            let receiver_stage = player.get_stage();

                                            drop(player);

                                            match (sender_stage.clone(), receiver_stage) {
                                                (Some(sender), Some(receiver))
                                                    if sender == receiver =>
                                                {
                                                    Some(packet)
                                                }

                                                _ => None,
                                            }
                                        }
                                    })
                                    .await;
                            }
                        });

                        false
                    }
                    Content::Player {
                        position: _,
                        quaternion: _,
                        animation_blend_weights: _,
                        act: _,
                        subact: _,
                    } if self.settings.read().await.flip_not_in(&packet.id) => {
                        let mut player = player.write().await;
                        player.last_position = Some(packet.content.clone());
                        player.loaded_save = true;
                        let sender_stage = player.get_stage();
                        drop(player);

                        tokio::spawn({
                            let server = self.clone();

                            let packet = packet.clone();

                            async move {
                                server
                                    .broadcast_map(packet, |player, packet| {
                                        let sender_stage = sender_stage.clone();

                                        async move {
                                            let player = player.read().await;
                                            let receiver_stage = player.get_stage();
                                            let size = player.size();
                                            drop(player);

                                            match (sender_stage, receiver_stage, packet.content) {
                                                (
                                                    Some(sender),
                                                    Some(receiver),
                                                    Content::Player {
                                                        position,
                                                        quaternion,
                                                        animation_blend_weights,
                                                        act,
                                                        subact,
                                                    },
                                                ) if sender == receiver => {
                                                    let position = position + Vec3::Y * size;
                                                    let quaternion = quaternion
                                                        * Quat::from_mat4(&Mat4::from_rotation_x(
                                                            std::f32::consts::PI,
                                                        ))
                                                        * Quat::from_mat4(&Mat4::from_rotation_y(
                                                            std::f32::consts::PI,
                                                        ));

                                                    Some(Packet::new(
                                                        id,
                                                        Content::Player {
                                                            position,
                                                            quaternion,
                                                            animation_blend_weights,
                                                            act,
                                                            subact,
                                                        },
                                                    ))
                                                }
                                                _ => None,
                                            }
                                        }
                                    })
                                    .await
                            }
                        });

                        false
                    }
                    Content::Player {
                        position: _,
                        quaternion: _,
                        animation_blend_weights: _,
                        act: _,
                        subact: _,
                    } => {
                        let mut player = player.write().await;
                        player.last_position = Some(packet.content.clone());
                        player.loaded_save = true;
                        let sender_stage = player.get_stage();
                        drop(player);

                        tokio::spawn({
                            let server = self.clone();

                            let packet = packet.clone();

                            async move {
                                server
                                    .broadcast_map(packet, |player, packet| {
                                        let sender_stage = sender_stage.clone();

                                        async move {
                                            let player = player.read().await;
                                            let receiver_stage = player.get_stage();
                                            drop(player);

                                            match (sender_stage, receiver_stage) {
                                                (Some(sender), Some(receiver))
                                                    if sender == receiver =>
                                                {
                                                    Some(packet)
                                                }
                                                _ => None,
                                            }
                                        }
                                    })
                                    .await
                            }
                        });

                        false
                    }
                    Content::Unknown => false,
                    _ => true,
                };

                if should_broadcast {
                    self.broadcast(packet).await;
                }
            }

            // TODO: Find out when peers & players are cleaned
            self.disconnect(id).await;

            Ok(())
        };

        match run().await {
            Ok(_) => Ok(()),
            Err(e) => {
                self.disconnect(id).await;
                Err(e)
            }
        }
    }

    async fn disconnect(&self, id: Uuid) {
        let mut peers = self.peers.write().await;
        let peer = peers.get_mut(&id);

        if peer.is_none() {
            return;
        }

        let mut peer = peer.unwrap();

        let player = self
            .players
            .get(&id)
            .await
            .expect("Player is supposed to be here");

        let player = player.read().await;
        peer.connected = false;
        peer.disconnect().await;
        drop(peers);
        self.broadcast(Packet::new(id, Content::Disconnect)).await;

        info!("{} just disconnected", player.name);
    }

    async fn on_new_peer(&self, peer: Peer) -> Result<Peer> {
        let settings = self.settings.read().await;

        let is_ip_banned = settings.ban_list.ips.iter().any(|addr| *addr == peer.ip);
        let is_id_banned = settings.ban_list.ids.iter().any(|addr| peer.id == *addr);

        drop(settings);

        if is_id_banned || is_ip_banned {
            info!(
                "Banned player {} with ip {} tried to joined",
                peer.ip, peer.id
            );

            Err(eyre!(
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
            .ok_or_else(|| eyre!("Couldn't find player"))?;

        let mut player = player.write().await;

        if player.is_speedrun {
            return Err(eyre!("Player is in speedrun mode"));
        }

        let bag = self.shine_bag.read().await;
        let peers = self.peers.read().await;
        let peer = peers.get(&id).ok_or_else(|| eyre!("Couldn't find peer"))?;

        for shine_id in bag.difference(&player.shine_sync.clone()) {
            player.shine_sync.insert(*shine_id);

            peer.send(Packet::new(id, Content::Shine { id: *shine_id }))
                .await
        }

        Ok(())
    }

    async fn persist_shines(&self) {
        let settings = self.settings.read().await;
        if !settings.persist_shines.enabled {
            return;
        }

        let shines = self.shine_bag.read().await;

        let shines = shines.clone();
        let file_name = settings.persist_shines.file_name.clone();

        drop(settings);

        let serialized = serde_json::to_string(&shines).unwrap();

        let _ = tokio::fs::write(file_name, serialized)
            .await
            .map_err(|err| {
                tracing::error!(%err, "Shine file failed to save");
                err
            });
    }

    pub async fn sync_shine_bag(&self) {
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

    pub async fn load_shines(&self) -> Result<()> {
        let settings = self.settings.read().await;

        if !settings.persist_shines.enabled {
            info!("Moon sync is disabled");
            return Ok(());
        }

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&settings.persist_shines.file_name)
            .await
            .expect("Moons couldn't be loaded or created");

        let mut content = String::from("");
        file.read_to_string(&mut content).await?;

        let deserialized = serde_json::from_str(&content).unwrap_or_default();

        let mut shines = self.shine_bag.write().await;

        info!("Moons loaded from {}", settings.persist_shines.file_name);

        drop(settings);

        *shines = deserialized;

        Ok(())
    }

    pub async fn disconnect_all(&self) {
        let peers = self.peers.read().await;

        join_all(peers.iter().map(|(_, peer)| peer.disconnect())).await;
    }

    pub async fn disconnect_by_name(&self, players: Vec<String>) {
        let ids = join_all(
            players
                .into_iter()
                .map(|name| self.players.get_id_by_name(name)),
        )
        .await
        .into_iter()
        .flatten();

        let mut peers = self.peers.write().await;

        for id in ids {
            let peer = peers.get_mut(&id);

            if peer.is_none() {
                continue;
            }

            let peer = peer.unwrap();

            peer.disconnect().await;
            peer.connected = false;
        }
    }
}

async fn receive_packet(reader: &mut ReadHalf<TcpStream>) -> Result<Packet> {
    let mut header_buf = [0; HEADER_SIZE];

    match reader.read_exact(&mut header_buf).await {
        Ok(n) if n == 0 => return Ok(Packet::new(Uuid::nil(), Content::Disconnect)),
        Ok(_) => (),
        Err(e) => {
            debug!("Connection closed: {}", e);
            return Ok(Packet::new(Uuid::nil(), Content::Disconnect));
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
            Ok(n) if n == 0 => return Err(eyre!("End of file reached")),
            Ok(_) => (),
            Err(e) => {
                debug!("Error reading header {}", e);
                return Err(eyre!(e));
            }
        };

        Bytes::from(body_buf)
    } else {
        Bytes::new()
    };

    header.make_packet(body)
}
