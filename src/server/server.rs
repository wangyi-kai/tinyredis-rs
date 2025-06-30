use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Semaphore};
use tokio::time;
use tracing::{debug, error, info};

use crate::server::connection::Connection;
use crate::db_engine::DbHandler;

const MAX_CONNECTIONS: usize = 250;

#[derive(Debug)]
struct Listener {
    listener: TcpListener,
    limit_connections: Arc<Semaphore>,
    db_handle: Arc<DbHandler>,
}

impl Listener {
    async fn run(&mut self) -> crate::Result<()> {
        loop {
            self.limit_connections.acquire().await.unwrap().forget();
            let socket = self.accept().await?;
            info!("accept new connection");
            let conn = Connection::new(socket);
        }

        Ok(())
    }


    async fn accept(&mut self) -> crate::Result<TcpStream> {
        let mut backoff = 1;
        loop {
            match self.listener.accept().await {
                Ok((socket, _)) => return Ok(socket),
                Err(err) => {
                    if backoff > 64 {
                        return Err(err.into());
                    }
                }
            }
            time::sleep(Duration::from_secs(backoff)).await;
            backoff *= 2;
        }
    }
}

struct Handler {
    connection: Connection,
    limit_connections: Arc<Semaphore>,
    _shutdown_complete: mpsc::Sender<()>,
    db_sender: crate::MpscSender,
    db_handler: Arc<DbHandler>,
}

impl Handler {
    async fn run(&mut self) -> crate::Result<()> {
        loop {
            let frame = tokio::select! {
                res = self.connection.read_frame() => res?,
                _ =
            };

        }
    }
}

