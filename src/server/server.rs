use std::future::Future;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Semaphore, broadcast, oneshot};
use tokio::time;
use tracing::{debug, error, info, Instrument};
use crate::cmd::command::RedisCommand;
use crate::server::connection::Connection;
use crate::db_engine::DbHandler;
use crate::parser::frame::Frame;
use crate::server::shutdown::Shutdown;

const MAX_CONNECTIONS: usize = 250;

#[derive(Debug)]
struct Listener {
    listener: TcpListener,
    notify_shutdown: broadcast::Sender<()>,
    limit_connections: Arc<Semaphore>,
    db_handler: Arc<DbHandler>,
    shutdown_complete_tx: mpsc::Sender<()>,
    shutdown_complete_rx: mpsc::Receiver<()>,
}

impl Listener {
    async fn run(&mut self) -> crate::Result<()> {
        loop {
            self.limit_connections.acquire().await.unwrap().forget();
            let socket = self.accept().await?;
            info!("accept new connection");
            let mut handler = Handler {
                connection: Connection::new(socket),
                limit_connections: self.limit_connections.clone(),
                shutdown: Shutdown::new(self.notify_shutdown.subscribe()),
                _shutdown_complete: self.shutdown_complete_tx.clone(),
                db_sender: self.db_handler.as_ref().get_sender(0).unwrap(),
                db_handler: self.db_handler.clone(),
            };
            tokio::spawn(async move {
                if let Err(err) = handler.run().await {
                    error!(cause = ?err, "handler error");
                }
            });
        }
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
    shutdown: Shutdown,
    _shutdown_complete: mpsc::Sender<()>,
    db_sender: crate::MpscSender,
    db_handler: Arc<DbHandler>,
}

impl Handler {
    async fn run(&mut self) -> crate::Result<()> {
        loop {
            let frame = tokio::select! {
                res = self.connection.read_frame() => res?,
                _ = self.shutdown.receiver() => return Ok(())
            };

            if let Some(frame) = frame {
                let result_cmd = RedisCommand::from_frame(frame);
                let result_frame = match result_cmd {
                    Ok(command) => match command {
                        RedisCommand::Hash(cmd) => {
                            let (sender, receiver) = oneshot::channel();
                            self.db_sender.send((sender, RedisCommand::Hash(cmd))).await?;
                            receiver.await?
                        }
                         _ => todo!()
                    }
                    Err(e) => return Err(e.into())
                };
                let frame = result_frame.unwrap_or_else(|e| Frame::Error(e.to_string()));
                self.connection.write_frame(&frame).await?;
            }
        }
    }
}

pub async fn run(listener: TcpListener, shutdown: impl Future, db_num: u32) {
    let (shutdown_complete_tx, shutdown_complete_rx) = mpsc::channel(1);
    let mut server = Listener {
        listener,
        notify_shutdown: broadcast::channel(1).0,
        limit_connections: Arc::new(Semaphore::new(MAX_CONNECTIONS)),
        db_handler: Arc::new(DbHandler::new(db_num)),
        shutdown_complete_tx,
        shutdown_complete_rx,
    };
    tokio::select! {
        res = server.run() => {
            if let Err(err) = res {
                 error!(cause = %err, "failed to accept");
            }
        },
        _ = shutdown => {
             info!("server shutting down");
       }
    }
    let Listener {
        mut shutdown_complete_rx,
        shutdown_complete_tx,
        notify_shutdown,
        ..
    } = server;
    drop(notify_shutdown);
    drop(shutdown_complete_tx);
    let _ = shutdown_complete_rx.recv().await;
}

