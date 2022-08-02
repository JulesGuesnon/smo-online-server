use colored::Colorize;
use server::Server;
use settings::Settings;
use std::{net::SocketAddr, str::FromStr, sync::Arc, time::Duration};
use tokio::{
    io::{self, AsyncWriteExt},
    net::TcpListener,
    time::sleep,
};
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;

mod commands;
mod packet;
mod peer;
mod players;
mod server;
mod settings;

#[tokio::main]
async fn main() -> io::Result<()> {
    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_str("info").unwrap())
        .finish();

    tracing::subscriber::set_global_default(subscriber).unwrap();

    let settings = Settings::load().await;

    let server = Arc::new(Server::new(settings));

    let _ = server.load_shines().await;

    let bind_address = SocketAddr::from_str(&format!(
        "{}:{}",
        server.settings.read().await.server.address.to_string(),
        server.settings.read().await.server.port
    ))
    .expect("Invalid address, please check address and port in settings.json");

    let listener = TcpListener::bind(bind_address).await?;

    tokio::spawn({
        let server = server.clone();

        async move {
            loop {
                sleep(Duration::from_secs(120)).await;

                server.sync_shine_bag().await;
            }
        }
    });

    tokio::spawn({
        let server = server.clone();
        async move { commands::listen(server).await }
    });

    info!("Server ready and listening on {}", bind_address);
    info!(
        "Write {} or {} to get the list of the available commands",
        "help".cyan(),
        "press enter".cyan(),
    );

    loop {
        let (mut socket, _) = listener.accept().await?;
        let server = server.clone();

        tokio::spawn(async move {
            if let Ok(addr) = socket.peer_addr() {
                let settings = server.settings.read().await;
                let is_banned = settings.ban_list.is_ip_ban(&addr.ip());
                drop(settings);

                if is_banned {
                    let _ = socket.shutdown().await;
                    return;
                }
            }

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
