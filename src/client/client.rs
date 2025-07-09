use tokio::net::{TcpStream, ToSocketAddrs};
use tracing::{debug, error, info, instrument};
use crate::client::config::Config;
use crate::cmd::command::{CommandStrategy, RedisCommand};
use crate::server::connection::Connection;
use crate::cmd::hash::HashCmd::{HDel, HGet, HSet};

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

pub struct Tokens {
    token: Vec<String>,
    index: usize,
}

impl Tokens {
    pub fn from(str: &String) -> Self {
        let mut is_str = false;
        let mut token = vec![];
        let mut temp = String::new();
        let chars = str.chars().into_iter().collect::<Vec<char>>();
        let mut i = 0;
        loop {
            match chars[i] {
                ' ' | '\n' | '\r' => if !is_str && !temp.is_empty() {
                    token.push(temp);
                    temp = String::new();
                } else {
                    temp.push(' ');
                }
                '\\' => if chars.len() > i + 1 && chars[i + 1].eq(&'"') {
                    i += 1;
                    temp.push('"')
                } else {
                    temp.push('\\')
                }
                '"' => is_str = !is_str,
                ';' => if !is_str {
                    if !temp.is_empty() {
                        token.push(temp);
                        break;
                    }
                },
                c => temp.push(c)
            }
            i += 1;
            if i >= chars.len() {
                if !temp.is_empty() { token.push(temp); }
                break;
            }
        }
        Self { token, index: 0 }
    }
    pub fn expect_next(&mut self, token: &str) -> bool {
        if self.index >= self.token.len() { return false; }
        if self.token[self.index].to_lowercase().eq(token) {
            self.index += 1;
            true
        } else {
            false
        }
    }

    pub fn expect_nexts(&mut self, tokens: Vec<&str>) -> bool {
        if self.index + tokens.len() > self.token.len() { return false; }
        for i in 0..tokens.len() {
            if !self.token[self.index + i].to_lowercase().eq(tokens[i]) {
                return false;
            }
        }
        self.index += tokens.len();
        true
    }
    pub fn next(&mut self) -> Option<String> {
        if self.index >= self.token.len() { return None; }
        let result = self.token[self.index].clone();
        self.index += 1;
        Some(result)
    }
    pub fn next_all(&mut self) -> Option<String> {
        if self.index >= self.token.len() { return None; }
        let result = self.token[self.index].clone();
        self.index += 1;
        if self.index == self.token.len() {
            Some(result)
        } else { None }
    }

    pub fn get_cmd(&self, index: usize) -> Option<&String> {
        self.token.get(index)
    }

    pub fn to_command(self) -> Option<RedisCommand> {
        let cmd_name = self.token[0].to_string();
        match &cmd_name[..] {
            "hset" => {
                let key = self.token[1].to_string();
                let field = self.token[2].to_string();
                let value = self.token[3].to_string();
                Some(RedisCommand::Hash(HSet {key, field, value}))
            }
            "hget" => {
                let key = self.token[1].to_string();
                let field = self.token[2].to_string();
                Some(RedisCommand::Hash(HGet {key, field}))
            }
            "hdel" => {
                let key = self.token[1].to_string();
                let field = self.token[2].to_string();
                Some(RedisCommand::Hash(HDel {key, field}))
            }
            _ => None
        }
    }
}

pub async fn run_client() -> crate::Result<()> {
    tracing_subscriber::fmt::try_init()?;
    let config = Config::new(None);
    let host = config.get_value("server_ip").unwrap().trim_matches('"').to_string();
    let port = config.get_value("server_port").unwrap();
    let addr = format!("{}:{}", host, port);

    let mut client = Client::connect(addr.clone()).await.unwrap();
    let mut command = String::new();
    'clear: loop {
        command.clear();
        println!("<{}>: ", addr);
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
            let tokens = Tokens::from(&command);
            let cmd = tokens.to_command().unwrap();
            let frame = cmd.into_frame();
            let _ = client.conn.write_frame(&frame).await;
            let res = client.conn.read_frame().await;
            match res {
                Ok(res) => println!("{}", res.unwrap()),
                Err(e) => println!("error: {}", e),
            };
            continue 'clear;
        }
    }
    Ok(())
}