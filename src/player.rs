use crate::packet::{Content, Packet};
use tokio::{
    io::{AsyncWriteExt, WriteHalf},
    net::TcpStream,
    sync::Mutex,
};
use uuid::Uuid;

#[derive(Debug, Default)]
pub struct Costume {
    pub body: String,
    pub cap: String,
}

pub struct Player {
    pub id: Uuid,
    pub connected: bool,
    pub costume: Option<Costume>,
    pub name: String,
    socket: Mutex<WriteHalf<TcpStream>>,
}

impl Player {
    pub fn new(socket: WriteHalf<TcpStream>) -> Self {
        Self {
            id: Uuid::new_v4(),
            connected: false,
            costume: None,
            name: "".to_string(),
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

    pub fn set_name(&mut self, new_name: String) {
        self.name = new_name;
    }

    pub fn set_id(&mut self, new_id: Uuid) {
        self.id = new_id;
    }

    pub fn set_costume(&mut self, body: String, cap: String) {
        self.costume = Some(Costume { body, cap });
    }
}
