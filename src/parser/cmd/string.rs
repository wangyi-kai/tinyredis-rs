use bytes::Bytes;
use tracing::info;
use crate::db::db::RedisDb;
use crate::db::object::{RedisObject, RedisValue};
use crate::parser::cmd::command::{CommandStrategy, RedisCommand};
use crate::parser::cmd::error::CommandError;
use crate::parser::cmd::error::CommandError::ObjectTypeError;
use crate::parser::frame::Frame;

#[allow(dead_code)]
#[derive(Debug)]
pub enum StringCmd {
    /// Appends a string to the value of a key. Creates the key if it doesn't exist
    Append { key: String, field: String },
    /// Returns the string value of a key
    Get { key: String},
    /// Sets the string value of a key, ignoring its type. The key is created if it doesn't exist
    SetEX { key: String, value: String, ttl: i128 },
    SetPX { key: String, value: String, ttl: i128 },
    SetNX { key: String, value: String },
    SetXX { key: String, value: String },
    /// Returns the length of a string value
    Strlen,
    /// Increments the integer value of a key by one
    Incr,
    /// Increments the integer value of a key by a number
    IncrBy,
    /// Decrements the integer value of a key by one
    Decr,
    /// Decrements a number from the integer value of a key
    DecrBy,
}

impl CommandStrategy for StringCmd {
    fn into_frame(self) -> Frame {
        let mut frame = Frame::Array(vec![]);
        match self {
            StringCmd::Append { key, field} => {
                frame.push_bulk(Bytes::from("append".as_bytes()));
                frame.push_bulk(Bytes::from(key.into_bytes()));
                frame.push_bulk(Bytes::from(field.into_bytes()));
            }
            StringCmd::SetEX {key, value, ttl} => {
                frame.push_bulk(Bytes::from("setex".as_bytes()));
                frame.push_bulk(Bytes::from(key.into_bytes()));
                frame.push_bulk(Bytes::from(value.into_bytes()));
                frame.push_bulk(Bytes::from(ttl.to_string().into_bytes()));
            }
            StringCmd::SetPX {key, value, ttl} => {
                frame.push_bulk(Bytes::from("setpx".as_bytes()));
                frame.push_bulk(Bytes::from(key.into_bytes()));
                frame.push_bulk(Bytes::from(value.into_bytes()));
                frame.push_bulk(Bytes::from(ttl.to_string().into_bytes()));
            }
            StringCmd::SetNX {key, value} => {
                frame.push_bulk(Bytes::from("setnx".as_bytes()));
                frame.push_bulk(Bytes::from(key.into_bytes()));
                frame.push_bulk(Bytes::from(value.into_bytes()));
            }
            StringCmd::SetXX {key, value} => {
                frame.push_bulk(Bytes::from("setxx".as_bytes()));
                frame.push_bulk(Bytes::from(key.into_bytes()));
                frame.push_bulk(Bytes::from(value.into_bytes()));
            }
            StringCmd::Get {key} => {
                frame.push_bulk(Bytes::from("get".as_bytes()));
                frame.push_bulk(Bytes::from(key.into_bytes()));
            }
            _ => return Frame::Null,
        }
        frame
    }

    fn from_frame(name: &str, frame: Frame) -> crate::Result<RedisCommand> {
        match name {
            "append" => {
                let key = frame.get_frame_by_index(1).ok_or("command error 'set'")?.to_string();
                let field = frame.get_frame_by_index(2).ok_or("command error 'set'")?.to_string();
                Ok(RedisCommand::String(StringCmd::Append {key, field}))
            }
            "setex" => {
                let key = frame.get_frame_by_index(1).ok_or("command error 'set'")?.to_string();
                let value = frame.get_frame_by_index(2).ok_or("command error 'set'")?.to_string();
                let ttl: i128 = frame.get_frame_by_index(3).ok_or("command error 'set'")?.to_string().parse()?;
                Ok(RedisCommand::String(StringCmd::SetEX {key, value, ttl: ttl * 1000}))
            }
            "setpx" => {
                let key = frame.get_frame_by_index(1).ok_or("command error 'set'")?.to_string();
                let value = frame.get_frame_by_index(2).ok_or("command error 'set'")?.to_string();
                let ttl: i128 = frame.get_frame_by_index(3).ok_or("command error 'set'")?.to_string().parse()?;
                Ok(RedisCommand::String(StringCmd::SetPX {key, value, ttl}))
            }
            "setnx" => {
                let key = frame.get_frame_by_index(1).ok_or("command error 'set'")?.to_string();
                let value = frame.get_frame_by_index(2).ok_or("command error 'set'")?.to_string();
                Ok(RedisCommand::String(StringCmd::SetNX {key, value}))
            }
            "setxx" => {
                let key = frame.get_frame_by_index(1).ok_or("command error 'set'")?.to_string();
                let value = frame.get_frame_by_index(2).ok_or("command error 'set'")?.to_string();
                Ok(RedisCommand::String(StringCmd::SetXX {key, value}))
            }
            "get" => {
                let key = frame.get_frame_by_index(1).ok_or("command error 'set'")?.to_string();
                Ok(RedisCommand::String(StringCmd::Get {key}))
            }
            _ => Err(CommandError::ParseError(-2).into())
        }
    }

    fn apply(self, db: &mut RedisDb<RedisObject<String>>) -> crate::Result<Frame> {
        match self {
            StringCmd::Append { key, field } => {
                let key = RedisObject::<String>::create_string_object(key);
                let mut o = db.lookup_key(&key);
                if let Some(o) = o {
                    return match &mut o.ptr {
                        RedisValue::String(s) => {
                            s.push_str(&field);
                            Ok(Frame::Simple("OK".to_string()))
                        }
                        _ => {
                            Err(ObjectTypeError(-1).into())
                        }
                    }
                } else {
                    let value = RedisObject::<String>::create_string_object(field);
                    db.add(key, value);
                    Ok(Frame::Simple("OK".to_string()))
                }
            },
            StringCmd::Get {key} => {
                let key = RedisObject::<String>::create_string_object(key);
                let mut o = db.lookup_key(&key);
                if let Some(o) = o {
                    match &o.ptr {
                        RedisValue::String(s) => {
                            Ok(Frame::Bulk(Bytes::from(s.clone().into_bytes())))
                        }
                        _ => Ok(Frame::Null)
                    }
                } else {
                    Ok(Frame::Null)
                }
            }
            _ => Err(CommandError::ParseError(-2).into())
        }
    }
}