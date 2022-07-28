use std::sync::Arc;

use server::Server;
use settings::Settings;
use tokio::{io, net::TcpListener};
use tracing::debug;

mod packet;
mod peer;
mod players;
mod server;
mod settings;

#[tokio::main]
async fn main() -> io::Result<()> {
    tracing_subscriber::fmt::init();

    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    let settings = Settings::default();

    let server = Arc::new(Server::new(settings));

    server.load_shines().await;

    loop {
        let (socket, _) = listener.accept().await?;
        let server = server.clone();

        tokio::spawn(async move {
            match socket.set_nodelay(true) {
                Ok(_) => {
                    let _ = server.handle_connection(socket).await;
                }
                Err(_) => {
                    debug!("Couldn't set NODELAY to socket, dropping it");
                    drop(socket)
                }
            };
        });
    }
}
