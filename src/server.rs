use crate::player;

use super::{
    packet::{ConnectionType, Content, Header, Packet, HEADER_SIZE},
    player::Player,
};
use anyhow::anyhow;
use anyhow::Result;
use bytes::Bytes;
use futures::future::join_all;
use std::collections::HashMap;
use tokio::{
    io::{split, AsyncReadExt, ReadHalf},
    net::TcpStream,
    sync::RwLock,
};
use uuid::Uuid;

use tracing::{debug, info};

const MAX_PLAYER: i16 = 10;

struct Server {
    players: RwLock<HashMap<Uuid, Player>>,
}

impl Server {
    pub fn new() -> Self {
        Self {
            players: RwLock::default(),
        }
    }

    async fn broadcast(&self, packet: Packet) {
        let players = self.players.read().await;

        join_all(
            players
                .iter()
                .filter(|(_, p)| p.connected && p.id != packet.id)
                .map(|(_, p)| p.send(packet.clone())),
        )
        .await;
    }

    pub async fn handle_connection(&self, socket: TcpStream) -> Result<()> {
        let (mut reader, writer) = split(socket);

        let mut player = Player::new(writer);
        let mut player_id = player.id.clone();

        player
            .send(Packet::new(
                player.id,
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

        let players = self.players.read().await;

        let connected_players = players
            .iter()
            .fold(0, |acc, p| if p.1.connected { acc + 1 } else { 0 });

        if connected_players == MAX_PLAYER {
            info!("Player {} couldn't join server is full", packet.id);
            return Err(anyhow!("Server full"));
        }

        drop(players);

        let mut players = self.players.write().await;

        // Remove stales clients and only keep the disconnected one
        let old_player = players.remove(&packet.id);

        match (packet.content, old_player) {
            (_, Some(old_player)) => {
                debug!(
                    "Found an old client {}, player {} reconnecting to it",
                    old_player.id, player.id
                );

                player.set_id(old_player.id);
                player.set_name(old_player.name);

                players.insert(packet.id, player);
                ()
            }
            (
                Content::Connect {
                    type_: ConnectionType::First,
                    max_player: _,
                    client,
                },
                None,
            ) => {
                debug!("Client {} with id {} is joining", client, player.id);
                player.set_id(packet.id);
                player_id = packet.id;
                player.set_name(client);

                players.insert(packet.id, player);
                ()
            }
            (
                Content::Connect {
                    type_: ConnectionType::Reconnect,
                    max_player: _,
                    client,
                },
                None,
            ) => {
                debug!(
                    "Client {} attempted to reconnect but there was no matching player",
                    player.id
                );
                player.set_name(client);
                players.insert(packet.id, player);
            }
            _ => {
                debug!("This case isn't supposed to be reach");
                return Err(anyhow!("This case isn't supposed to be reach"));
            }
        }

        let players = self.players.read().await;

        let player = players
            .get(&player_id)
            .ok_or(anyhow!("Player is supposed to be in the HashMap"))?;

        for (uuid, p) in self.players.read().await.iter() {
            if *uuid != player_id {
                let _ = player
                    .send(Packet::new(
                        p.id,
                        Content::Connect {
                            type_: ConnectionType::First,
                            max_player: MAX_PLAYER as u16,
                            client: p.name.clone(),
                        },
                    ))
                    .await;

                if let Some(costume) = &p.costume {
                    let _ = player
                        .send(Packet::new(
                            p.id,
                            Content::Costume {
                                body: costume.body.clone(),
                                cap: costume.cap.clone(),
                            },
                        ))
                        .await;
                }
            }
        }

        drop(player);
        drop(players);

        loop {
            let packet = receive_packet(&mut reader).await?;

            // TODO: Implement packet handler
            match &packet.content {
                Content::Costume { body, cap } => {
                    let mut players = self.players.write().await;
                    let player = players
                        .get_mut(&player_id)
                        .ok_or(anyhow!("Player is supposed to be in the HashMap"))?;

                    player.set_costume(body.clone(), cap.clone());
                }
                _ => (),
            }

            self.broadcast(packet).await;
        }

        Ok(())
    }
}

async fn receive_packet(reader: &mut ReadHalf<TcpStream>) -> Result<Packet> {
    let mut header_buf = [0; HEADER_SIZE];

    match reader.read_exact(&mut header_buf).await {
        Ok(n) if n == 0 => return Err(anyhow!("End of file reached")),
        Ok(n) => (),
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
            Ok(n) => (),
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
