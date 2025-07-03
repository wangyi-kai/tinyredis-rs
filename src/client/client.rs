use tokio::net::{TcpStream, ToSocketAddrs};
use crate::server::connection::Connection;
use crate::cmd::hash::HashCmd;

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

pub async fn run_client() {
    let mut client = Client::connect("127.0.0.1:6379").await.unwrap();
    let mut cmd = HashCmd::HSet {
            key: "hello".to_string(),
            field: "world1".to_string(),
            value: "world2".to_string(),
        };
    let frame = cmd.into_frame();
    let _ = client.conn.write_frame(&frame).await;
    let res = client.conn.read_frame().await;
    match res {
        Ok(res) => println!("res: {}", res.unwrap()),
        Err(e) => println!("error: {}", e),
    };

    // let mut command = String::new();
    // 'clear: loop {
    //     command.clear();
    //     println!("<{}>: ", "127.0.0.1:6379");
    //     'cmd: loop {
    //          std::io::stdin().read_line(&mut command).unwrap();
    //         if command.ends_with("\n") {
    //             command.remove(command.len() - 1);
    //         }
    //         if command.ends_with("\r") {
    //             command.remove(command.len() - 1);
    //         }
    //         if !command.ends_with(";") {
    //             command.push_str("\n");
    //             continue 'cmd;
    //         }
    //
    //     }
    // }
}