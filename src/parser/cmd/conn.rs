use bytes::Bytes;
use crate::parser::cmd::command::{RedisCommand};
use crate::parser::cmd::conn::ConnCmd::{Echo, Ping, Select};
use crate::parser::cmd::error::CommandError;
use crate::parser::frame::Frame;
use crate::server::server::Handler;

#[derive(Debug, Clone)]
pub enum ConnCmd {
    /// Authenticates the connection
    Auth,
    /// A container for client connection commands
    Client,
    /// Returns the given string
    Echo { msg: String },
    /// Handshakes with the Redis server
    Hello,
    /// Returns the server's liveliness response
    Ping {msg: Option<String> },
    /// Closes the connection
    Quit,
    /// Resets the connection
    Reset,
    /// Changes the selected database
    Select { index: usize },
}

impl  ConnCmd {
    pub fn into_frame(self) -> Frame {
        let mut frame = Frame::Array(vec![]);
        match self {
            Echo {msg} => {
                frame.push_bulk(Bytes::from("echo".as_bytes()));
                frame.push_bulk(Bytes::from(msg.into_bytes()));
                frame
            }
            Ping {msg} => {
                frame.push_bulk(Bytes::from("ping".as_bytes()));
                if let Some(msg) = msg {
                    frame.push_bulk(Bytes::from(msg.into_bytes()));
                }
                frame
            }
            Select {index} => {
                frame.push_bulk(Bytes::from("select".as_bytes()));
                frame.push_bulk(Bytes::from(index.to_string().into_bytes()));
                frame
            }
            _ => Frame::Null
        }
    }

    pub fn from_frame(name: &str, frame: Frame) -> crate::Result<RedisCommand> {
        match name {
            "echo" => {
                let msg = frame.get_frame_by_index(1).ok_or("command error 'echo'")?.to_string();
                Ok(RedisCommand::Connection(Echo {msg}))
            }
            "ping" => {
                let msg = frame.get_frame_by_index(1);
                if let Some(msg) = msg {
                    Ok(RedisCommand::Connection(Ping {msg: Some(msg.to_string())}))
                } else {
                    Ok(RedisCommand::Connection(Ping { msg: None }))
                }
            }
            "select" => {
                let index: usize = frame.get_frame_by_index(1).ok_or("command error 'select'")?.to_string().parse()?;
                Ok(RedisCommand::Connection(Select {index}))
            }
            _ => Err(CommandError::ParseError(-4).into())
        }
    }

    pub fn apply(&self, handler: &mut Handler) -> crate::Result<Frame> {
        match self {
            Echo {msg} => {
                Ok(Frame::Simple(msg.clone()))
            }
            Ping {msg} => {
                if let Some(msg) = msg {
                    Ok(Frame::Simple(msg.clone()))
                } else {
                    Ok(Frame::Simple("pong".to_string()))
                }
            }
            Select {index} => {
                handler.change_db(*index)?;
                Ok(Frame::Simple(format!("change db{}", index)))
            }
            _ => Err(CommandError::ParseError(-3).into())
        }
    }
}