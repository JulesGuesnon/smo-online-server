use tokio::{
    io,
    net::{TcpListener, TcpStream},
};
use tracing::debug;

mod game;
mod packet;
mod player;
mod server;

#[tokio::main]
async fn main() -> io::Result<()> {
    tracing_subscriber::fmt::init();

    let listener = TcpListener::bind("127.0.0.1:8080").await?;

    loop {
        let (socket, _) = listener.accept().await?;

        tokio::spawn(async move {
            match socket.set_nodelay(true) {
                Ok(_) => handle_connection(socket).await,
                Err(_) => {
                    debug!("Couldn't set NODELAY to socket, dropping it");
                    drop(socket)
                }
            };
        });
    }
}

async fn handle_connection(mut socket: TcpStream) {
    let (reader, writer) = socket.split();
}
