use crate::packet::Packet;
use std::net::SocketAddr;
use tokio::{
    io::{AsyncWriteExt, WriteHalf},
    net::TcpStream,
    sync::Mutex,
};
use uuid::Uuid;

#[derive(Debug)]
pub struct Peer {
    pub id: Uuid,
    pub ip: SocketAddr,
    pub connected: bool,
    socket: Mutex<WriteHalf<TcpStream>>,
}

// Player -> Player
// State related stuff -> Game state: Arc<RwLock<HashMap<Uuid, RwLock<State>>>>
impl Peer {
    pub fn new(ip: SocketAddr, socket: WriteHalf<TcpStream>) -> Self {
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
