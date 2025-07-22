use crate::parser::cmd::error::CommandError;
use crate::parser::cmd::hash::HashCmd;
use crate::parser::frame::Frame;
use crate::parser::cmd::string::StringCmd;
use crate::db::db::RedisDb;
use crate::db::object::RedisObject;
use crate::parser::cmd::conn::ConnCmd;

pub trait CommandStrategy {
    fn into_frame(self) -> Frame;
    fn from_frame(name: &str, frame: Frame) -> crate::Result<RedisCommand>;
    fn apply(self, db: &mut RedisDb<RedisObject<String>>) -> crate::Result<Frame>;
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum RedisCommand {
    Connection(ConnCmd),
    String(StringCmd),
    List(ListCmd),
    Set(SetCmd),
    SortSet(SortedCmd),
    Hash(HashCmd),
}

impl CommandStrategy for RedisCommand {
    fn into_frame(self) -> Frame {
        match self {
            RedisCommand::Hash(cmd) => cmd.into_frame(),
            RedisCommand::String(cmd) => cmd.into_frame(),
            RedisCommand::Connection(cmd) => cmd.into_frame(),
            _ => unimplemented!()
        }
    }

    fn from_frame(_name: &str, frame: Frame) -> crate::Result<RedisCommand> {
        let cmd_name = get_command_name(&frame).ok().unwrap().to_lowercase();

        let command = match &cmd_name[..] {
            "hset" | "hget" | "hdel" =>
                HashCmd::from_frame(&cmd_name, frame)?,
            "append" | "set" | "get" | "setex" | "setnx" | "setpx" | "setxx" | "strlen" =>
                StringCmd::from_frame(&cmd_name, frame)?,
            "select" | "echo" | "ping" | "quit" => ConnCmd::from_frame(&cmd_name, frame)?,
            _ => return Err(CommandError::ParseError(-101).into()),
        };
        Ok(command)
    }

    fn apply(self, db: &mut RedisDb<RedisObject<String>>) -> crate::Result<Frame> {
        match self {
            RedisCommand::Hash(cmd) => cmd.apply(db),
            RedisCommand::String(cmd) => cmd.apply(db),
            _ => unimplemented!()
        }
    }
}

pub fn parse_frame(frame_vec: Vec<Frame>) -> crate::Result<Vec<RedisCommand>> {
    let mut cmd_vec= Vec::with_capacity(1024);
    for frame in frame_vec {
        let cmd = RedisCommand::from_frame("", frame)?;
        cmd_vec.push(cmd);
    }
    Ok(cmd_vec)
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum ListCmd {
    /// Prepends one or more elements to a list. Creates the key if it doesn't exist
    LPush,
    /// Appends one or more elements to a list. Creates the key if it doesn't exist
    RPush,
    /// Returns the first elements in a list after removing it. Deletes the list if the last element was popped
    LPop,
    /// Returns and removes the last elements of a list. Deletes the list if the last element was popped
    RPop,
    /// Sets the value of an element in a list by its index
    LSet,
    /// Inserts an element before or after another element in a list
    LInsert,
    /// Returns the length of a list
    LLen,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum SetCmd {
    /// Returns the number of members in a set
    SCard,
    /// Adds one or more members to a set
    SAdd,
    /// Iterates over members of a set
    SSCan,
    /// Returns the union of multiple sets
    SUnion,
    /// Returns the intersect of multiple sets
    SInter,
    /// Returns the number of members of the intersect of multiple sets
    SInterCard,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum SortedCmd {
    /// Adds one or more members to a sorted set, or updates their scores.
    ZAdd,
    /// Returns the number of members in a sorted set
    ZCard,
    /// Returns the score of a member in a sorted set
    ZScore,
    /// Returns the union of multiple sorted sets
    ZUnion,
    /// Returns the intersect of multiple sorted sets
    ZInter,
    /// Returns the number of members of the intersect of multiple sorted sets
    ZInterCard,
    /// Stores the intersect of multiple sorted sets in a key
    ZInterStore,
}

pub fn get_command_name(frame: &Frame) -> crate::Result<String> {
    match frame.get_frame_by_index(0).ok_or("frame is empty")? {
        Frame::Simple(s) => Ok(s.clone()),
        Frame::Bulk(bytes) => {
            let str = std::str::from_utf8(&bytes[..])?;
            Ok(String::from(str))
        }
        _ => Err("frame is error type".into()),
    }
}