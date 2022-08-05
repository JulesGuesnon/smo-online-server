#![forbid(unsafe_code)]
#![deny(private_in_public)]
#![warn(
    clippy::all,
    clippy::dbg_macro,
    clippy::todo,
    clippy::empty_enum,
    clippy::enum_glob_use,
    clippy::unused_self,
    clippy::needless_continue,
    clippy::needless_borrow,
    clippy::match_wildcard_for_single_variants,
    clippy::if_let_mutex,
    clippy::mismatched_target_os,
    clippy::match_on_vec_items,
    clippy::imprecise_flops,
    clippy::suboptimal_flops,
    clippy::lossy_float_literal,
    clippy::fn_params_excessive_bools,
    clippy::inefficient_to_string,
    clippy::macro_use_imports,
    clippy::option_option,
    clippy::unnested_or_patterns,
    clippy::str_to_string,
    clippy::cast_lossless,
    clippy::implicit_clone,
    clippy::unused_async,
    clippy::redundant_closure_for_method_calls,
    rust_2018_idioms,
    future_incompatible,
    nonstandard_style,
    missing_debug_implementations
)]

use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use owo_colors::OwoColorize;
use server::Server;
use settings::Settings;
use tokio::io::{self, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::time::sleep;
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
        server.settings.read().await.server.address,
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
