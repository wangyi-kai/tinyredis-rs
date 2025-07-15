use bytes::Bytes;
use crate::cmd::command::{CommandStrategy, RedisCommand};
use crate::cmd::error::CommandError;
use crate::data_structure::dict::dict::Value;
use crate::data_structure::dict::dict::Value::Val;
use crate::db::db::RedisDb;
use crate::object::{OBJ_ENCODING_HT, RedisObject, RedisValue};
use crate::parser::frame::Frame;

#[derive(Debug)]
pub enum HashCmd {
    /// Creates or modifies the value of a field in a hash
    HSet { key: String, field: String, value: String},
    /// Returns the value of a field in a hash
    HGet { key: String, field: String },
    /// Deletes one or more fields and their values from a hash.
    HDel { key: String, field: String },
    /// Iterates over fields and values of a hash
    HScan,
}

impl CommandStrategy for HashCmd {
    fn into_frame(self) -> Frame {
        let mut frame = Frame::Array(vec![]);
        match self {
            HashCmd::HSet { key, field, value } => {
                frame.push_bulk(Bytes::from("hset".as_bytes()));
                frame.push_bulk(Bytes::from(key.into_bytes()));
                frame.push_bulk(Bytes::from(field.into_bytes()));
                frame.push_bulk(Bytes::from(value.into_bytes()));
                frame
            }
            HashCmd::HGet { key, field } => {
                frame.push_bulk(Bytes::from("hget".as_bytes()));
                frame.push_bulk(Bytes::from(key.into_bytes()));
                frame.push_bulk(Bytes::from(field.into_bytes()));
                frame
            }
            HashCmd::HDel { key, field } => {
                frame.push_bulk(Bytes::from("hdel".as_bytes()));
                frame.push_bulk(Bytes::from(key.into_bytes()));
                frame.push_bulk(Bytes::from(field.into_bytes()));
                frame
            }
            HashCmd::HScan => Frame::Null
        }
    }

    fn from_frame(name: &str, frame: Frame) -> crate::Result<RedisCommand> {
        let len = frame.get_len();
        match name {
            "hset" => {
                let key = frame.get_frame_by_index(1).ok_or("command error 'set'")?.to_string();
                let field = frame.get_frame_by_index(2).ok_or("command error 'set'")?.to_string();
                let value = frame.get_frame_by_index(3).ok_or("command error 'set'")?.to_string();
                Ok(RedisCommand::Hash(HashCmd::HSet { key, field, value }))
            },
            "hget" => {
                let key = frame.get_frame_by_index(1).ok_or("command error 'set'")?.to_string();
                let field = frame.get_frame_by_index(2).ok_or("command error 'set'")?.to_string();
                Ok(RedisCommand::Hash(HashCmd::HGet { key, field }))
            },
            "hdel" => {
                let key = frame.get_frame_by_index(1).ok_or("command error 'set'")?.to_string();
                let field = frame.get_frame_by_index(2).ok_or("command error 'set'")?.to_string();
                Ok(RedisCommand::Hash(HashCmd::HDel { key, field }))
            },
            _ => Err(CommandError::ParseError(-1).into())
        }
    }

    fn apply(self, db: &mut RedisDb<RedisObject<String>>) -> crate::Result<Frame> {
        match self {
            HashCmd::HGet { key, field } => {
                let key = RedisObject::<String>::create_string_object(key);
                let mut o = db.lookup_key(&key);
                if let Some(o) = o {
                    let val = Self::hash_get(o, &field);
                    if let Some(val) = val {
                        Ok(Frame::Bulk(val.into()))
                    } else {
                        Ok(Frame::Null)
                    }
                } else {
                    Ok(Frame::Null)
                }
            }
            HashCmd::HDel { key, field } => {
                let key = RedisObject::<String>::create_string_object(key);
                let mut o = db.lookup_key(&key);
                if let Some(o) = o {
                    Self::hash_delete(o, &field);
                }
                Ok(Frame::Simple("ok".to_string()))
            }
            HashCmd::HSet { key, field, value } => {
                let key = RedisObject::<String>::create_string_object(key);
                let mut o = db.lookup_key(&key);
                if let Some(o) = o {
                    Self::hash_set(o, field, value);
                } else {
                    let mut ht = RedisObject::<String>::create_hash_object();
                    Self::hash_set(&mut ht, field, value);
                    db.add(key, ht);
                }
                Ok(Frame::Simple("OK".to_string()))
            }
            HashCmd::HScan => todo!()
        }
    }
}

impl HashCmd {
    fn hash_set(o: &mut RedisObject<String>, field: String, value: String) {
        if o.encoding == OBJ_ENCODING_HT {
            let mut ht = match &mut o.ptr {
                RedisValue::Hash(ht) => ht,
                _ => return,
            };
            let entry = ht.find(&field);
            unsafe {
                if let Some(entry) = entry {
                    (*entry.as_ptr()).val = Some(value)
                } else {
                    ht.add_raw(field, value).ok();
                }
            }
        } else {
            todo!()
        }
    }

    fn hash_get(o: &mut RedisObject<String>, field: &str) -> Option<&'static str> {
        if o.encoding == OBJ_ENCODING_HT {
            let de = match &mut o.ptr {
                RedisValue::Hash(ht) => ht.find(field),
                _ => return None
            };
            if let Some(de) = de {
                unsafe {
                    let value = (*de.as_ptr()).value();
                    Some(value)
                }
            } else {
                None
            }
        } else {
            todo!()
        }
    }

    fn hash_delete(o: &mut RedisObject<String>, field: &str) -> bool {
        let mut deleted = false;
        if o.encoding == OBJ_ENCODING_HT {
            match &mut o.ptr {
                RedisValue::Hash(ht) => {
                    ht.generic_delete(field).ok();
                    true
                }
                _ => deleted,
            }
        } else {
            deleted
        }
    }
}

#[cfg(test)]
mod test {
    use crate::cmd::command::{CommandStrategy, get_command_name};
    use crate::cmd::hash::HashCmd;

    #[test]
    fn cmd_to_frame() -> crate::Result<()> {
        let mut cmd = HashCmd::HSet {
            key: "hello".to_string(),
            field: "world1".to_string(),
            value: "world2".to_string(),
        };
        let frame = cmd.into_frame();
        println!("frame {}", frame);

        let cmd_type = get_command_name(&frame)?;
        let cmd = HashCmd::from_frame(&cmd_type, frame)?;

        println!("cmd_type:{}, cmd: {:?}", cmd_type, cmd);
        Ok(())
    }
}