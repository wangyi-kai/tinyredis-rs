use std::io::Write;
use std::io::Read;
use bytes::{Buf, BufMut, BytesMut};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc::Sender;

use crate::db::data_structure::dict::dict::Value;
use crate::db::db_engine::RDbCommand;
use crate::persistence::error::PersistError;
use crate::db::object::{*};
use crate::parser::cmd::hash::HashCmd;
use crate::parser::cmd::zset::SortedCmd;
use crate::persistence::{*};
use crate::{Result};

pub enum RdbCommand {
    Save { db_id: u32, sender: std::sync::mpsc::Sender<Result<()>> },
    Load { sender: std::sync::mpsc::Sender<Result<()>> },
}

#[derive(Clone)]
pub struct RdbHandler {
    sender: std::sync::mpsc::Sender<RdbCommand>,
}

impl RdbHandler {
    pub fn new(db_sender: Vec<Sender<RDbCommand>>) -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<RdbCommand>();
        let mut rdb = Rdb::create(db_sender);

        std::thread::spawn(move || {
            loop {
                match rx.recv() {
                    Ok(RdbCommand::Save { db_id, sender }) => {
                        let _ = sender.send(rdb.save(db_id));
                    }
                    Ok(RdbCommand::Load {sender}) => {
                        let _ = sender.send(rdb.load());
                    }
                    Err(e) => {
                        tracing::error!("rdb channel err: {:?}", e);
                        break;
                    }
                }
            }
        });

        Self { sender: tx }
    }

    pub fn save(&self, db_id: u32) -> Result<()> {
        let (tx, rx) = std::sync::mpsc::channel::<Result<()>>();
        let _ = self.sender.send(RdbCommand::Save { db_id, sender: tx });
        rx.recv().map_err(|e| PersistError::RdbErr(e.to_string()))?
    }

    pub fn load(&self) -> Result<()> {
        let (tx, rx) = std::sync::mpsc::channel::<Result<()>>();
        let _ = self.sender.send(RdbCommand::Load { sender: tx });
        rx.recv().map_err(|e| PersistError::RdbErr(e.to_string()))?
    }
}


struct Rdb {
    db_sender: Vec<Sender<RDbCommand>>,
}

impl Rdb {
    pub fn create(db_sender: Vec<Sender<RDbCommand>>) -> Self {
        Self { db_sender, }
    }

    pub fn save(&mut self, db_id: u32) -> Result<()> {
        let tmp_path = "./tmp.rdb".to_string();
        let mut tmp_file = std::fs::File::options()
            .read(true)
            .write(true)
            .create(true)
            .open(&tmp_path).map_err(|_| PersistError::FileError(-101))?;
        let mut buf = BytesMut::with_capacity(1024 * 8);

        buf.extend_from_slice(b"RDB");
        buf.put_u8(RDB_OPCODE_SELECTDB);
        Self::rdb_save_len(&mut buf, db_id as u64)?;
        let (tx, rx) = std::sync::mpsc::channel();
        let cmd = RDbCommand::DbIter(tx);
        let sender = self.db_sender[db_id as usize].clone();
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                let _ = sender.send(cmd).await;
            });
        unsafe {
            match rx.recv() {
                Ok(iter) => {
                    for dict in iter {
                        let key = (*dict).get_key();
                        let value = (*dict).get_val();
                        match value {
                            Value::Val(robj) => {
                                Self::rdb_save_key_value_pair(&mut buf, key, robj)?;
                            },
                            _ => {}
                        }
                    }
                    buf.put_u8(RDB_OPCODE_EOF);
                }
                _ => { }
            }
        };

        tmp_file.write_all(&mut buf).map_err(|_| PersistError::FileError(-102))?;
        let rdb_path = "./dump.rdb".to_string();
        std::fs::rename(tmp_path, rdb_path).map_err(|_| PersistError::FileError(-103))?;

        Ok(())
    }

    pub fn load(&mut self) -> Result<()> {
        let rdb_path = "./dump.rdb".to_string();
        let mut file = std::fs::File::options()
            .read(true)
            .open(rdb_path)
            .map_err(|_| PersistError::FileError(-104))?;
        let mut buf_vec = Vec::with_capacity(1024 * 8);
        file.read_to_end(&mut buf_vec).map_err(|_| PersistError::FileError(-105))?;
        let mut buf = BytesMut::from(&buf_vec[..]);
        let head = buf.split_to(3);
        if head != b"RDB"[..] {
            return Err(PersistError::DecodeErr("flag not rdb".to_string()).into());
        }
        let db_id_flag = buf.get_u8();
        if db_id_flag != RDB_OPCODE_SELECTDB {
            return Err(PersistError::DecodeErr("db_id_flag error".to_string()).into());
        }
        let db_id = Self::rdb_load_len(&mut buf)?;
        println!("db_id: {}", db_id);
        let sender = self.db_sender[db_id as usize].clone();
        
        // 解析数据并在当前异步上下文中发送命令
        loop {
            if buf.is_empty() {
                break;
            }
            match buf.get_u8() {
                RDB_OPCODE_EOF => break,
                RDB_TYPE_STRING => {
                    let s = Self::load_string(&mut buf)?;
                    let key = RedisObject::create_string_object(s);
                    let value = Self::rdb_load_object(RDB_TYPE_STRING, &mut buf)?;
                    let cmd = RDbCommand::RdbData {key, value};
                    tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()?
                        .block_on(async {
                            let _ = sender.send(cmd).await.map_err(|_| PersistError::FileError(-106));
                        });
                },
                RDB_TYPE_HASH => {
                    let s = Self::load_string(&mut buf)?;
                    let key = RedisObject::create_string_object(s);
                    let value = Self::rdb_load_object(RDB_TYPE_HASH, &mut buf)?;
                    let cmd = RDbCommand::RdbData {key, value};
                    tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()?
                        .block_on(async {
                            let _ = sender.send(cmd).await.map_err(|_| PersistError::FileError(-107));
                        });
                }
                RDB_TYPE_ZSET_2 => {
                    let s = Self::load_string(&mut buf)?;
                    let key = RedisObject::create_string_object(s);
                    let value = Self::rdb_load_object(RDB_TYPE_ZSET_2, &mut buf)?;
                    let cmd = RDbCommand::RdbData {key, value};
                    tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()?
                        .block_on(async {
                            let _ = sender.send(cmd).await.map_err(|_| PersistError::FileError(-108));
                        });
                }
                _ => {
                    return Err(PersistError::DecodeErr("invalid rdb load byte".to_string()).into());
                }
            }
        }

        Ok(())
    }

    #[inline(always)]
    fn rdb_save_object_type(buf: &mut BytesMut, object: &RedisObject) -> Result<()> {
        match object.object_type {
            OBJ_STRING => {
                buf.put_u8(RDB_TYPE_STRING);
            }
            OBJ_SET => {
                match object.encoding {
                    OBJ_ENCODING_INTSET => buf.put_u8(RDB_TYPE_SET_INTSET),
                    OBJ_ENCODING_HT => buf.put_u8(RDB_TYPE_SET),
                    _ => return Err(PersistError::EncodeErr("Unknown set encoding".to_string()).into())
                }
            }
            OBJ_ZSET => {
                match object.encoding {
                    OBJ_ENCODING_SKIPLIST => buf.put_u8(RDB_TYPE_ZSET_2),
                    _ => return Err(PersistError::EncodeErr("Unknown sorted set encoding".to_string()).into())
                }
            }
            OBJ_HASH => {
                match object.encoding {
                    OBJ_ENCODING_HT => buf.put_u8(RDB_TYPE_HASH),
                    OBJ_ENCODING_ZIPLIST => buf.put_u8(RDB_TYPE_HASH_ZIPLIST),
                    _ => return Err(PersistError::EncodeErr("Unknown hash encoding".to_string()).into())
                }
            }
            _ => {
                return Err(PersistError::EncodeErr("Unknown object type".to_string()).into())
            }
        }
        Ok(())
    }

    fn rdb_save_string(buf: &mut BytesMut, s: &str) -> Result<usize> {
        let s_vec = s.as_bytes();
        let len = s_vec.len();
        Self::rdb_save_len(buf, len as u64)?;
        buf.put_slice(s_vec);
        Ok(len)
    }

    #[inline(always)]
    fn rdb_save_len(buf: &mut BytesMut, len: u64) -> Result<usize> {
        let mut nwritten = 0;
        if len < 1 << 6 {
            buf.put_u8(len as u8 | RDB_6BITLEN << 6);
            nwritten = 1;
        } else if len < 1 << 14 {
            buf.put_u8(((len >> 8) as u8) & 0xFF | (RDB_14BITLEN << 6));
            buf.put_u8((len & 0xFF) as u8);
            nwritten = 2;
        } else if len <= u32::MAX as u64 {
            buf.put_u8(RDB_32BITLEN);
            buf.put_u32(len as u32);
            nwritten = 1 + 4;
        } else {
            buf.put_u8(RDB_64BITLEN);
            buf.put_u64(len);
            nwritten = 1 + 8;
        }
        Ok(nwritten)
    }

    fn rdb_save_key_value_pair(buf: &mut BytesMut, key: &str, value: &RedisObject) -> Result<()> {
        println!("key: {}, encoding: {}", key, value.encoding);
        Self::rdb_save_object_type(buf, value)?;
        Self::rdb_save_string(buf, key)?;
        Self::rdb_save_object(buf, value)?;
        Ok(())
    }

    fn rdb_save_object(buf: &mut BytesMut, object: &RedisObject) -> Result<()> {
        let mut nwritten = 0;
        match object.object_type {
            OBJ_STRING => {
                match &object.ptr {
                    RedisValue::String(s) => {
                        let n = Self::rdb_save_string(buf, s)?;
                        nwritten += n;
                    }
                    _ => {
                        return Err(PersistError::TypeErr("err object type, expect string".to_string()).into())
                    }
                }
            }
            OBJ_HASH => {
                match &object.ptr {
                    RedisValue::Hash(ht) => {
                        let ht_iter = ht.iter();
                        let size = ht.dict_size();
                        nwritten += Self::rdb_save_len(buf, size as u64)?;
                        unsafe {
                            for entry in ht_iter {
                                let field = (*entry).get_key();
                                println!("save field: {}", field);
                                nwritten += Self::rdb_save_string(buf, field)?;
                                let value = (*entry).value();
                                match value {
                                    Value::Sds(s) => {
                                        println!("save value: {}", s);
                                        nwritten += Self::rdb_save_string(buf, s)?
                                    },
                                    _ => {}
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            OBJ_ZSET => {
                match &object.ptr {
                    RedisValue::SortSet(zset) => {
                        let zsl = &zset.zsl;
                        let len = zsl.length;
                        nwritten += Self::rdb_save_len(buf, len)?;
                        let mut zn = zsl.tail;
                        unsafe {
                            while let Some(node) = zn {
                                nwritten += Self::rdb_save_string(buf, &node.as_ref().get_elem())?;
                                buf.put_f64(node.as_ref().get_score());
                                zn = node.as_ref().back_ward();
                            }
                        }
                    }
                    _ => {
                        return Err(PersistError::TypeErr("err object type, expect zset".to_string()).into())
                    }
                }
            }
            _ => { }
        }
        Ok(())
    }

    fn rdb_load_object(obj_type: u8, buf: &mut BytesMut) -> Result<RedisObject> {
        match obj_type {
            RDB_TYPE_STRING => {
                let s = Self::load_string(buf)?;
                let object = RedisObject::create_string_object(s);
                Ok(object)
            }
            RDB_TYPE_HASH => {
                let hash_size = Self::rdb_load_len(buf)?;
                let mut object = RedisObject::create_hash_object();
                for _ in 0..hash_size {
                    let key = Self::load_string(buf)?;
                    let value = Self::load_string(buf)?;
                    HashCmd::hash_set(&mut object, key, value);
                }
                Ok(object)
            }
            RDB_TYPE_ZSET_2 => {
                let mut object = RedisObject::create_zset_object();
                let len = Self::rdb_load_len(buf)?;
                for _ in 0..len {
                    let ele = Self::load_string(buf)?;
                    let score = buf.get_f64();
                    SortedCmd::zadd(&mut object, None, score, ele);
                }
                Ok(object)
            }
            _ => {
                Err(PersistError::TypeErr("obj_type".to_string()).into())
            }
        }
    }

    #[inline(always)]
    fn rdb_load_len(buf: &mut BytesMut) -> Result<u64> {
        let len_type = buf.get_u8();
        let len = match (len_type & 0xC0) >> 6 {
            RDB_ENCVAL | RDB_6BITLEN => {
                (len_type & 0x3F) as u64
            }
            RDB_14BITLEN => {
                let mut res = ((len_type & 0x3F) as u64) << 8;
                res |= buf.get_u8() as u64;
                res
            }
            RDB_32BITLEN => {
                buf.get_u32() as u64
            }
            RDB_64BITLEN => {
                buf.get_u64()
            }
            _ => {
                return Err(PersistError::LoadErr(format!("Unknown length encoding {} in rdb_load_len", len_type)).into());
            }
        };

        Ok(len)
    }

    fn load_string(buf: &mut BytesMut) -> Result<String> {
        let len = Self::rdb_load_len(buf)?;
        let s = String::from_utf8(buf
            .split_to(len as usize)
            .to_vec())
            .map_err(|_| PersistError::DecodeErr("invalid utf-8".to_string()))?;

        Ok(s)
    }
}

