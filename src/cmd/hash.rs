use crate::cmd::command::{get_command_name, RedisCommand};
use crate::cmd::error::CommandError;
use crate::data_structure::dict::dict::Value;
use crate::db::db::RedisDb;
use crate::object::{OBJ_ENCODING_HT, RedisObject, RedisValue};
use crate::parser::frame::Frame;

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

impl HashCmd {
    pub fn from_frame(command_name: &str, frame: Frame) -> Result<HashCmd, CommandError> {
        let len = frame.get_len();
        match command_name {
            "hset" => {
                let key = frame.get_frame_by_index(1).ok_or("command error 'set'")?.to_string();
                let field = frame.get_frame_by_index(2).ok_or("command error 'set'")?.to_string();
                let value = frame.get_frame_by_index(3).ok_or("command error 'set'")?.to_string();
                Ok(HashCmd::HSet {key, field, value})
            },
            "hget" => {
                let key = frame.get_frame_by_index(1).ok_or("command error 'set'")?.to_string();
                let field = frame.get_frame_by_index(2).ok_or("command error 'set'")?.to_string();
                Ok(HashCmd::HGet {key, field})
            },
            "hdel" => {
                let key = frame.get_frame_by_index(1).ok_or("command error 'set'")?.to_string();
                let field = frame.get_frame_by_index(2).ok_or("command error 'set'")?.to_string();
                Ok(HashCmd::HDel {key, field})
            },
            _ => Err(CommandError::ParseError(-1))
    }
}
    pub fn apply(self, db: &mut RedisDb<RedisObject<String>>) -> crate::Result<Frame> {
        match self {
            HashCmd::HGet {key, field} => {
                let key = RedisObject::create_string_object(key);
                let mut o = db.lookup_key(&key);
                if o.is_some() {
                    let val = Self::hash_get(o.unwrap(), &field);
                    Ok(Frame::Bulk(val.clone().into()))
                } else {
                    Ok(Frame::Null)
                }
            }
            HashCmd::HDel {key, field} => {
                let key = RedisObject::create_string_object(key);
                let mut o = db.lookup_key(&key);
                if o.is_some() {
                    Self::hash_delete(o.unwrap(), &field);
                }
                Ok(Frame::Simple("ok".to_string()))
            }
            HashCmd::HSet {key, field, value} => {
                let key = RedisObject::create_string_object(key);
                let mut o = db.lookup_key(&key);
                if o.is_none() {
                    let mut ht = RedisObject::create_hash_object();
                    Self::hash_set(&mut ht, field, value);
                    db.add(key, ht);
                } else {
                    Self::hash_set(o.unwrap(), field, value);
                }
                Ok(Frame::Simple("ok".to_string()))
            }
            HashCmd::HScan => todo!()
        }
    }

    fn hash_set(o: &mut RedisObject<String>, field: String, value: String) {
        if o.encoding == OBJ_ENCODING_HT {
            let mut ht = match &mut o.ptr {
                RedisValue::Hash(ht) => ht,
                _ => return,
            };
            let entry = ht.find(&field);
            unsafe {
                if entry.is_some() {
                    (*entry.unwrap().as_ptr()).val = value;
                } else {
                    ht.add_raw(field, value).ok();
                }
            }
        } else {
            todo!()
        }
    }

    fn hash_get(o: &mut RedisObject<String>, field: &str) -> &'static str {
        if o.encoding == OBJ_ENCODING_HT {
            let de = match &mut o.ptr {
                RedisValue::Hash(ht) => ht.find(&field),
                _ => return &"".to_string()
            };
            unsafe {
                let value = &(*de.unwrap().as_ptr()).val;
                value
            }
        } else {
            todo!()
        }
    }

    fn hash_delete(o: &mut RedisObject<String>, field: &str) -> bool {
        let mut deleted = false;
        if o.encoding == OBJ_ENCODING_HT {
            return match &mut o.ptr {
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