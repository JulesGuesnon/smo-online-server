use crate::packet::Packet;
use tokio::{
    io::{AsyncWriteExt, WriteHalf},
    net::TcpStream,
    sync::Mutex,
};
use uuid::Uuid;

pub struct Peer {
    pub id: Uuid,
    pub connected: bool,
    socket: Mutex<WriteHalf<TcpStream>>,
}

// Player -> Player
// State related stuff -> Game state: Arc<RwLock<HashMap<Uuid, RwLock<State>>>>
impl Peer {
    pub fn new(socket: WriteHalf<TcpStream>) -> Self {
        Self {
            id: Uuid::new_v4(),
            connected: false,
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

        // TODO: Handle error
        let _ = socket.write_all(&packet.serialize()).await;
    }
}
