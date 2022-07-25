use super::packet::Packet;
use tokio::{io::AsyncWriteExt, net::tcp::WriteHalf, sync::Mutex};
use uuid::Uuid;

#[derive(Debug, Default)]
pub struct Costume {
    body: String,
    cap: String,
}

pub struct Player<'a> {
    pub id: Uuid,
    pub connected: bool,
    pub costume: Costume,
    pub name: String,
    socket: Mutex<WriteHalf<'a>>,
}

impl<'a> Player<'a> {
    pub fn new(socket: WriteHalf<'a>) -> Self {
        Self {
            id: Uuid::new_v4(),
            connected: false,
            costume: Costume::default(),
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
}
