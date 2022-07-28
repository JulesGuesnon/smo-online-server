use std::{net::SocketAddr, str::FromStr, sync::Arc};

use server::Server;
use settings::Settings;
use tokio::{io, net::TcpListener};
use tracing::{debug, info};

mod packet;
mod peer;
mod players;
mod server;
mod settings;

#[tokio::main]
async fn main() -> io::Result<()> {
    tracing_subscriber::fmt::init();

    let settings = Settings::load().await;

    let server = Arc::new(Server::new(settings));

    let _ = server.load_shines().await;

    let bind_address = SocketAddr::from_str(&format!(
        "{}:{}",
        server.settings.server.address.to_string(),
        server.settings.server.port
    ))
    .unwrap();

    let listener = TcpListener::bind(bind_address).await?;

    loop {
        let (socket, _) = listener.accept().await?;
        let server = server.clone();

        tokio::spawn(async move {
            match socket.set_nodelay(true) {
                Ok(_) => match server.handle_connection(socket).await {
                    Ok(_) => (),
                    Err(message) => info!("handle_connection exited with error: {}", message),
                },
                Err(_) => {
                    debug!("Couldn't set NODELAY to socket, dropping it");
                    drop(socket)
                }
            };
        });
    }
}
