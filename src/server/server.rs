use std::future::Future;
use std::sync::{Arc};
use std::sync::atomic::AtomicU64;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Semaphore, broadcast, oneshot};
use tokio::time;
use tracing::{error, info};

use crate::config::ServerConfig;
use crate::parser::cmd::command::{CommandStrategy, RedisCommand};
use crate::parser::cmd::conn::{*};
use crate::server::connection::Connection;
use crate::db::db_engine::DbHandler;
use crate::parser::frame::Frame;
use crate::persistence::rdb::RdbHandler;
use crate::server::{REDIS_CONFIG, REDIS_SERVER};
use crate::server::shutdown::Shutdown;

const MAX_CONNECTIONS: usize = 250;

#[derive(Debug)]
pub struct RedisServer {
    listener: TcpListener,
    notify_shutdown: broadcast::Sender<()>,
    limit_connections: Arc<Semaphore>,
    db_handler: Arc<DbHandler>,
    shutdown_complete_tx: mpsc::Sender<()>,
    shutdown_complete_rx: mpsc::Receiver<()>,
    pub(crate) dirty: AtomicU64,
}

impl RedisServer {
    pub fn new(listener: TcpListener) -> Self {
        let (shutdown_complete_tx, shutdown_complete_rx) = mpsc::channel(1);
        let db_num = REDIS_CONFIG.get().unwrap().db_num;
        let db_handler = Arc::new(DbHandler::new(db_num));
        let db_sender = db_handler.db_sender.clone();
        let rdb_handler = RdbHandler::new(db_sender);
        match rdb_handler.load() {
            Ok(_) => info!("load rdb file success"),
            Err(err) => error!(cause = %err, "load rdb file failed")
        }
        let save_params = REDIS_CONFIG.get().unwrap().get_param();
        if !save_params.is_empty()  {
            for param in save_params {
                let handler = rdb_handler.clone();
                tokio::spawn(async move {
                    Self::run_rdb_check(handler, param.seconds, param.changes as u64).await;
                });
            }
        }

        Self {
            listener,
            notify_shutdown: broadcast::channel(1).0,
            limit_connections: Arc::new(Semaphore::new(MAX_CONNECTIONS)),
            db_handler,
            shutdown_complete_tx,
            shutdown_complete_rx,
            dirty: AtomicU64::new(0),
        }
    }

    async fn run(&mut self) -> crate::Result<()> {
        info!("ready to accept connection");
        loop {
            self.limit_connections.acquire().await?.forget();
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

    pub fn incr_dirty(&mut self) {
        self.dirty.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    async fn run_rdb_check(rdb_handler: RdbHandler, seconds: u64, change: u64) {
        let mut interval = time::interval(Duration::from_secs(seconds));
        unsafe {
            loop {
                interval.tick().await;
                let current_dirty = REDIS_SERVER.get().unwrap().dirty.load(std::sync::atomic::Ordering::Relaxed);
                if current_dirty >= change {
                    let db_num = REDIS_CONFIG.get().unwrap().db_num;
                    for db_id in 0..db_num {
                        if let Err(e) = rdb_handler.save(db_id) {
                            error!("Failed to save RDB for database {}: {}", db_id, e);
                        }
                    }
                    REDIS_SERVER.get().unwrap().dirty.store(0, std::sync::atomic::Ordering::Relaxed);
                }
            }
        }
    }
}

pub struct Handler {
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
                let result_cmd = RedisCommand::from_frame("", frame)?;
                match &result_cmd {
                    RedisCommand::Connection(cmd) => {
                        match cmd {
                            ConnCmd::Quit => {
                                self.shutdown.shutdown();
                                return Ok(());
                            }
                            _ => {
                                let result = cmd.apply(self)?;
                                self.connection.write_frame(&result).await?;
                                continue;
                            }
                        }
                    }
                    _ => {
                        let (sender, receiver) = oneshot::channel();
                        self.db_sender.send((sender, result_cmd)).await?;
                        let frame = receiver.await?.unwrap_or_else(|e| Frame::Error(e.to_string()));
                        self.connection.write_frame(&frame).await?;
                    }
                };
            }
        }
    }

    pub fn change_db(&mut self, index: usize) -> crate::Result<()> {
        let sender = self.db_handler.get_sender(index).ok_or("ERR invalid DB index")?;
        self.db_sender = sender;
        Ok(())
    }

    pub fn shutdown(&mut self) {
        self.shutdown.shutdown();
    }
}

impl Drop for Handler {
    fn drop(&mut self) {
        info!("handler quit");
        self.limit_connections.add_permits(1);
    }
}

pub async unsafe fn run_server(listener: TcpListener, shutdown: impl Future, db_num: u32) {
    let mut server_config = ServerConfig::default();
    server_config.set_rdb_save_param(1, 1);
    REDIS_CONFIG.set(server_config).expect("set redis config failed");
    let server = RedisServer::new(listener);
    REDIS_SERVER.set(server).expect("set redis server failed");
    tokio::select! {
        res = REDIS_SERVER.get_mut().unwrap().run() => {
            if let Err(err) = res {
                 error!(cause = %err, "failed to accept");
            }
        },
        _ = shutdown => {
             info!("server shutting down");
       }
    }

    if let Some(server) = REDIS_SERVER.take() {
        let RedisServer {
            mut shutdown_complete_rx,
            shutdown_complete_tx,
            notify_shutdown,
            ..
        } = server;
        drop(notify_shutdown);
        drop(shutdown_complete_tx);
        let _ = shutdown_complete_rx.recv().await;
    }
}

