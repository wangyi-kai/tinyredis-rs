use tokio::net::{TcpStream, ToSocketAddrs};
use crate::connection::Connection;
use crate::db::db::RedisDb;
use crate::object::RedisObject;

pub struct Client {
    conn: Connection,
}

impl Client {
    pub async fn connect<A: ToSocketAddrs>(addr: A) -> crate::Result<Client> {
        let socket = TcpStream::connect(addr).await?;
        let connection = Connection::new(socket);

        Ok(Client { conn: connection })
    }
}