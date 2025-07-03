use tokio::net::{TcpStream, ToSocketAddrs};
use crate::server::connection::Connection;
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

pub fn run_client() {
    let mut command = String::new();
    'clear: loop {
        command.clear();
        'cmd: loop {
             std::io::stdin().read_line(&mut command).unwrap();
            if command.ends_with("\n") {
                command.remove(command.len() - 1);
            }
            if command.ends_with("\r") {
                command.remove(command.len() - 1);
            }
            if !command.ends_with(";") {
                command.push_str("\n");
                continue 'cmd;
            }
        }
    }
}