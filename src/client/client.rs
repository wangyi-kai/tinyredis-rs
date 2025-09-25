use tokio::io::AsyncWriteExt;
use tokio::net::{TcpStream, ToSocketAddrs};
use crate::parser::cmd::error::CommandError::{*};
use crate::parser::cmd::command::{RedisCommand};
use crate::server::connection::Connection;
use crate::parser::cmd::hash::HashCmd::{HDel, HGet, HSet};
use crate::parser::cmd::string::StringCmd::{*};
use crate::parser::cmd::conn::ConnCmd::{*};
use crate::parser::cmd::zset::SortedCmd::{ZAdd, ZCard, ZScore};
use crate::parser::frame::Frame;

pub struct Client {
    pub conn: Connection,
}

impl Client {
    pub async fn connect<A: ToSocketAddrs>(addr: A) -> crate::Result<Client> {
        let socket = TcpStream::connect(addr).await?;
        let connection = Connection::new(socket);

        Ok(Client { conn: connection })
    }

    pub async fn benchmark_send_command(&mut self, mut buf: Vec<u8>) -> crate::Result<()>
    {
        self.conn.stream.write_all(buf.as_mut_slice()).await?;
        self.conn.stream.flush().await?;
        Ok(())
    }

    pub async fn benchmark_receive(&mut self) -> crate::Result<Option<Frame>> {
        self.conn.read_frame().await
    }
}

pub struct Tokens {
    token: Vec<String>,
    index: usize,
}

impl Tokens {
    pub fn from(str: &str) -> Self {
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
                ';' => if !is_str && !temp.is_empty() {
                    token.push(temp);
                    break;
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

    pub fn to_command(self) -> crate::Result<RedisCommand> {
        let cmd_name = self.token[0].to_string();
        match &cmd_name[..] {
            "hset" => {
                let key = self.token[1].to_string();
                let field = self.token[2].to_string();
                let value = self.token[3].to_string();
                Ok(RedisCommand::Hash(HSet {key, field, value}))
            }
            "hget" => {
                if self.token.len() != 3 {
                    return Err(ArgsErr(cmd_name).into())
                }
                let key = self.token[1].to_string();
                let field = self.token[2].to_string();
                Ok(RedisCommand::Hash(HGet {key, field}))
            }
            "hdel" => {
                if self.token.len() != 3 {
                    return Err(ArgsErr(cmd_name).into())
                }
                let key = self.token[1].to_string();
                let field = self.token[2].to_string();
                Ok(RedisCommand::Hash(HDel {key, field}))
            }
            "append" => {
                let key = self.token[1].to_string();
                let field = self.token[2].to_string();
                Ok(RedisCommand::String(Append { key, field }))
            }
            "get" => {
                if self.token.len() != 2 {
                    return Err(ArgsErr(cmd_name).into())
                }
                let key = self.token[1].to_string();
                Ok(RedisCommand::String(Get {key}))
            }
            "set" => {
                if self.token.len() != 3 {
                    return Err(ArgsErr(cmd_name).into())
                }
                let key = self.token[1].to_string();
                let value = self.token[2].to_string();
                Ok(RedisCommand::String(Set {key, value}))
            }
            "setex" => {
                let key = self.token[1].to_string();
                let ttl:i128 = self.token[2].to_string().parse()?;
                Ok(RedisCommand::String(SetEX {key, ttl: ttl * 1000}))
            }
            "setpx" => {
                let key = self.token[1].to_string();
                let ttl:i128 = self.token[2].to_string().parse()?;
                Ok(RedisCommand::String(SetPX {key, ttl}))
            }
            "setnx" => {
                let key = self.token[1].to_string();
                let value = self.token[2].to_string();
                Ok(RedisCommand::String(SetNX {key, value}))
            }
            "setxx" => {
                let key = self.token[1].to_string();
                let value = self.token[2].to_string();
                Ok(RedisCommand::String(SetXX {key, value}))
            }
            "strlen" => {
                let s = self.token[1].to_string();
                Ok(RedisCommand::String(Strlen {s}))
            }
            "ping" => {
                if self.token.len() > 1 {
                    let s = self.token[1].to_string();
                    Ok(RedisCommand::Connection(Ping { msg: Some(s)}))
                } else {
                    Ok(RedisCommand::Connection(Ping { msg: None }))
                }
            }
            "echo" => {
                let s = self.token[1].to_string();
                Ok(RedisCommand::Connection(Echo {msg: s}))
            }
            "select" => {
                let idx: usize = self.token[1].to_string().parse()?;
                Ok(RedisCommand::Connection(Select {index: idx}))
            }
            "quit" => {
                Ok(RedisCommand::Connection(Quit))
            }
            "zadd" => {
                let key = self.token[1].to_string();
                let param = self.token[2].to_string();
                let (arg, start) = if param.eq("nx") || param.eq("xx") || param.eq("incr") || param.eq("lt") || param.eq("gt") {
                    (Some(param), 3)
                } else {
                    (None, 2)
                };
                let len = self.token.len();
                let mut values = Vec::with_capacity(len);
                for i in start..len {
                    values.push(self.token[i].clone())
                }
                Ok(RedisCommand::SortSet(ZAdd {key, arg, values}))
            }
            "zcard" => {
                if self.token.len() != 2 {
                    return Err(ArgsErr("zcard".to_string()).into())
                }
                let key = self.token[1].to_string();
                Ok(RedisCommand::SortSet(ZCard {key}))

            }
            "zscore" => {
                if self.token.len() != 3 {
                    return Err(ArgsErr("zscore".to_string()).into())
                }
                let key = self.token[1].to_string();
                let member = self.token[2].to_string();
                Ok(RedisCommand::SortSet(ZScore {key, member}))
            }
            _ => Err(NotSupport(cmd_name).into())
        }
    }
}