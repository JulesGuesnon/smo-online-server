// use std::{net::SocketAddr, str::FromStr, sync::Arc, time::Duration};

// use server::Server;
// use settings::Settings;
use tokio::{io, net::TcpListener, time::sleep};
// use tracing::{debug, info};

mod commands;
mod packet;
mod peer;
mod players;
mod server;
mod settings;

#[tokio::main]
async fn main() -> io::Result<()> {
    println!("{}", commands::Stage::help());
    // commands::listen().await;

    Ok(())
    // tracing_subscriber::fmt::init();

    // let settings = Settings::load().await;

    // let server = Arc::new(Server::new(settings));

    // let _ = server.load_shines().await;

    // let bind_address = SocketAddr::from_str(&format!(
    //     "{}:{}",
    //     server.settings.server.address.to_string(),
    //     server.settings.server.port
    // ))
    // .expect("Invalid address, please check address and port in settings.json");

    // let listener = TcpListener::bind(bind_address).await?;

    // tokio::spawn({
    //     let server = server.clone();

    //     async move {
    //         loop {
    //             sleep(Duration::from_secs(120)).await;

    //             server.sync_shine_bag().await;
    //         }
    //     }
    // });
    // loop {
    //     let (socket, _) = listener.accept().await?;
    //     let server = server.clone();

    //     tokio::spawn(async move {
    //         match socket.set_nodelay(true) {
    //             Ok(_) => match server.handle_connection(socket).await {
    //                 Ok(_) => (),
    //                 Err(message) => info!("handle_connection exited with error: {}", message),
    //             },
    //             Err(_) => {
    //                 debug!("Couldn't set NODELAY to socket, dropping it");
    //                 drop(socket)
    //             }
    //         };
    //     });
    // }
}
