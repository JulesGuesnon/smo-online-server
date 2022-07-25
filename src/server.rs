use super::{
    packet::{ConnectionType, Content, Header, Packet, HEADER_SIZE},
    player::Player,
};
use anyhow::Result;
use bytes::Bytes;
use tokio::{io::AsyncReadExt, net::TcpStream, sync::RwLock};
use tracing::{debug, info};

const MAX_PLAYER: i16 = 10;

struct Server<'a> {
    players: RwLock<Vec<Player<'a>>>,
}

impl<'a> Server<'a> {
    pub fn new() -> Self {
        Self {
            players: RwLock::default(),
        }
    }

    pub async fn handle_connection(&self, mut socket: TcpStream) -> Result<()> {
        let (mut reader, writer) = socket.split();

        let mut player = Player::new(writer);

        player
            .send(Packet::new(
                player.id,
                Content::Init {
                    max_player: MAX_PLAYER,
                },
            ))
            .await;

        let mut first = true;

        loop {
            let mut header_buf = [0; HEADER_SIZE];

            match reader.read_exact(&mut header_buf).await {
                Ok(n) if n == 0 => break,
                Ok(n) => (),
                Err(e) => {
                    debug!("Error reading header {}", e);
                    break;
                }
            };

            let header = match Header::from_bytes(Bytes::from(header_buf.to_vec())) {
                Ok(h) => h,
                Err(_) => break,
            };

            let body = if header.packet_size > 0 {
                let mut body_buf = vec![0; header.packet_size];

                match reader.read_exact(&mut body_buf).await {
                    Ok(n) if n == 0 => break,
                    Ok(n) => (),
                    Err(e) => {
                        debug!("Error reading header {}", e);
                        break;
                    }
                };

                Bytes::from(body_buf)
            } else {
                Bytes::new()
            };

            let packet = header.make_packet(body)?;

            if first {
                first = false;

                if !packet.content.is_connect() {
                    debug!(
                        "Player {} didn't send connection packet on first connection",
                        packet.id
                    );
                    break;
                }

                let players = self.players.read().await;

                let connected_players =
                    players.iter().fold(
                        0,
                        |acc, player| if player.connected { acc + 1 } else { acc },
                    );

                if connected_players == MAX_PLAYER {
                    info!("Player {} couldn't join server is full", packet.id);
                    break;
                }

                let find_player = players.iter().find(|player| player.id == header.id);

                // TODO: THIS PART
                match packet.content {
                    Content::Connect {
                        type_: ConnectionType::First,
                        max_player,
                        client,
                    } => (),
                    Content::Connect {
                        type_: ConnectionType::Reconnect,
                        max_player,
                        client,
                    } => {
                        player.set_id(packet.id);
                    }
                    _ => {
                        debug!("This case isn't supposed to be reach");
                        break;
                    }
                }
                let players = self.players.write().await;
            }
        }

        Ok(())
    }
}
