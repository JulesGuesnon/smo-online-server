use std::net::IpAddr;

use tokio::io::{AsyncWriteExt, WriteHalf};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::packet::Packet;

#[derive(Debug)]
pub struct Peer {
    pub id: Uuid,
    pub ip: IpAddr,
    pub connected: bool,
    socket: Mutex<WriteHalf<TcpStream>>,
}

// Player -> Player
// State related stuff -> Game state: Arc<RwLock<HashMap<Uuid, RwLock<State>>>>
impl Peer {
    #[inline]
    pub fn new(ip: IpAddr, socket: WriteHalf<TcpStream>) -> Self {
        Self {
            id: Uuid::nil(),
            ip,
            connected: true,
            socket: Mutex::new(socket),
        }
    }

    pub async fn disconnect(&self) {
        let mut socket = self.socket.lock().await;

        // TODO: Handle error
        let _ = socket.shutdown().await;
    }

    pub async fn send(&self, packet: Packet) {
        let mut socket = self.socket.lock().await;

        let _ = socket.write_all(&packet.as_bytes()).await;
    }
}
