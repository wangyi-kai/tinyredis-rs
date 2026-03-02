use std::{io::SeekFrom};
use tokio::{fs::File};
use bytes::{Buf, BufMut, BytesMut};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::sync::mpsc::Sender;
use crate::db::data_structure::dict::dict::Value;
use crate::db::db_engine::DbCommand;
use crate::persistence::error::PersistError;
use crate::db::object::{*};
use crate::parser::cmd::hash::HashCmd;
use crate::parser::cmd::zset::SortedCmd;
use crate::persistence::{*};
use crate::Result;

#[derive(Clone)]
pub struct Rdb {
    db_sender: Vec<Sender<DbCommand>>,
    buf: BytesMut,
}

impl Rdb {
    pub fn create(db_sender: Vec<Sender<DbCommand>>) -> Self {
        let buf = BytesMut::with_capacity(1024 * 8);
        Self { db_sender, buf }
    }

    pub async fn save(&mut self, db_id: u32) -> Result<()> {
        let tmp_path = "./tmp.rdb".to_string();
        let mut tmp_file = File::create(&tmp_path).await?;

        tmp_file.seek(SeekFrom::Start(0)).await?;
        self.buf.extend_from_slice(b"RDB");
        self.buf.put_u8(RDB_OPCODE_SELECTDB);
        self.rdb_save_len(db_id as u64)?;
        let (tx, mut rx) = tokio::sync::mpsc::channel(1);
        let cmd = DbCommand::DbIter(tx);
        let sender = self.db_sender[db_id as usize].clone();
        let _ = sender.send(cmd).await;
        unsafe {
            match rx.recv().await {
                Some(iter) => {
                    for dict in iter {
                        let key = (*dict).get_key();
                        let value = (*dict).get_val();
                        match value {
                            Value::Val(robj) => self.rdb_save_key_value_pair(key, robj)?,
                            _ => {}
                        }
                    }
                    self.buf.put_u8(RDB_OPCODE_EOF);
                }
                _ => { }
            }
        }

        tmp_file.write_all(&self.buf).await?;
        let rdb_path = "./dump.rdb".to_string();
        tokio::fs::rename(&tmp_path, &rdb_path).await?;

        Ok(())
    }

    pub async fn load(&mut self) -> Result<()> {
        let rdb_path = "./dump.rdb".to_string();
        let mut rdb_file = File::open(&rdb_path).await?;
        let mut buf = BytesMut::with_capacity(1024 * 8);
        loop {
            if rdb_file.read_buf(&mut buf).await? == 0 {
                break;
            }
        }
        let head = buf.split_to(3);
        if head != b"RDB"[..] {
            return Err(PersistError::DecodeErr("flag not rdb".to_string()).into());
        }
        let db_id_flag = buf.get_u8();
        if db_id_flag != RDB_OPCODE_SELECTDB {
            return Err(PersistError::DecodeErr("db_id_flag error".to_string()).into());
        }
        let db_id = self.rdb_load_len(&mut buf)?;
        let sender = self.db_sender[db_id as usize].clone();
        loop {
            match buf.get_u8() {
                RDB_OPCODE_EOF => break,
                RDB_TYPE_STRING => {
                    let s = self.load_string(&mut buf)?;
                    let key = RedisObject::create_string_object(s);
                    let value = self.rdb_load_object(RDB_TYPE_STRING, &mut buf)?;
                    let cmd = DbCommand::RdbData {key, value};
                    let _ = sender.send(cmd).await;
                },
                RDB_TYPE_HASH => {
                    let s = self.load_string(&mut buf)?;
                    let key = RedisObject::create_string_object(s);
                    let value = self.rdb_load_object(RDB_TYPE_HASH, &mut buf)?;
                    let cmd = DbCommand::RdbData {key, value};
                    let _ = sender.send(cmd).await;
                }
                RDB_TYPE_ZSET_2 => {
                    let s = self.load_string(&mut buf)?;
                    let key = RedisObject::create_string_object(s);
                    let value = self.rdb_load_object(RDB_TYPE_ZSET_2, &mut buf)?;
                    let cmd = DbCommand::RdbData {key, value};
                    let _ = sender.send(cmd).await;
                }
                _ => {
                    return Err(PersistError::DecodeErr("invalid rdb load byte".to_string()).into());
                }
            }
        }
        Ok(())
    }

    #[inline(always)]
    fn rdb_save_object_type(&mut self, object: &RedisObject) -> Result<()> {
        match object.object_type {
            OBJ_STRING => {
                self.buf.put_u8(RDB_TYPE_STRING);
            }
            OBJ_SET => {
                match object.encoding {
                    OBJ_ENCODING_INTSET => self.buf.put_u8(RDB_TYPE_SET_INTSET),
                    OBJ_ENCODING_HT => self.buf.put_u8(RDB_TYPE_SET),
                    _ => return Err(PersistError::EncodeErr("Unknown set encoding".to_string()).into())
                }
            }
            OBJ_ZSET => {
                match object.encoding {
                    OBJ_ENCODING_SKIPLIST => self.buf.put_u8(RDB_TYPE_ZSET_2),
                    _ => return Err(PersistError::EncodeErr("Unknown sorted set encoding".to_string()).into())
                }
            }
            OBJ_HASH => {
                match object.encoding {
                    OBJ_ENCODING_HT => self.buf.put_u8(RDB_TYPE_HASH),
                    _ => return Err(PersistError::EncodeErr("Unknown hash encoding".to_string()).into())
                }
            }
            _ => {
                return Err(PersistError::EncodeErr("Unknown object type".to_string()).into())
            }
        }
        Ok(())
    }

    fn rdb_save_string(&mut self, s: &str) -> Result<usize> {
        let s_vec = s.as_bytes();
        let len = s_vec.len();
        self.rdb_save_len(len as u64)?;
        self.buf.put_slice(s_vec);
        Ok(len)
    }


    #[inline(always)]
    fn rdb_save_len(&mut self, len: u64) -> Result<usize> {
        let mut nwritten = 0;
        if len < 1 << 6 {
            self.buf.put_u8(len as u8);
            nwritten = 1;
        } else if len < 1 << 14 {
            self.buf.put_u8(((len >> 8) as u8) | (RDB_14BITLEN << 6));
            self.buf.put_u8(len as u8);
            nwritten = 2;
        } else if len <= u32::MAX as u64 {
            self.buf.put_u8(RDB_32BITLEN);
            self.buf.put_u32(len as u32);
            nwritten = 1 + 4;
        } else {
            self.buf.put_u8(RDB_6BITLEN);
            self.buf.put_u64(len);
            nwritten = 1 + 8;
        }
        Ok(nwritten)
    }

    fn rdb_save_key_value_pair(&mut self, key: &str, value: &RedisObject) -> Result<()> {
        self.rdb_save_object_type(value)?;
        self.rdb_save_string(key)?;
        println!("save key: {}", key);
        self.rdb_save_object(value)?;
        Ok(())
    }

    fn rdb_save_object(&mut self, object: &RedisObject) -> Result<()> {
        let mut nwritten = 0;
        match object.object_type {
            OBJ_STRING => {
                match &object.ptr {
                    RedisValue::String(s) => {
                        let n = self.rdb_save_string(s)?;
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
                        nwritten += self.rdb_save_len(size as u64)?;
                        unsafe {
                            for entry in ht_iter {
                                let field = (*entry).get_key();
                                println!("save field: {}", field);
                                nwritten += self.rdb_save_string(field)?;
                                let value = (*entry).value();
                                match value {
                                    Value::Sds(s) => {
                                        println!("save value: {}", s);
                                        nwritten += self.rdb_save_string(s)?
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
                        nwritten += self.rdb_save_len(len)?;
                        let mut zn = zsl.tail;
                        unsafe {
                            while let Some(node) = zn {
                                nwritten += self.rdb_save_string(&node.as_ref().get_elem())?;
                                self.buf.put_f64(node.as_ref().get_score());
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

    fn rdb_load_object(&self, obj_type: u8, buf: &mut BytesMut) -> Result<RedisObject> {
        match obj_type {
            RDB_TYPE_STRING => {
                let s = self.load_string(buf)?;
                let object = RedisObject::create_string_object(s);
                Ok(object)
            }
            RDB_TYPE_HASH => {
                let hash_size = self.rdb_load_len(buf)?;
                let mut object = RedisObject::create_hash_object();
                for _ in 0..hash_size {
                    let key = self.load_string(buf)?;
                    let value = self.load_string(buf)?;
                    HashCmd::hash_set(&mut object, key, value);
                }
                Ok(object)
            }
            RDB_TYPE_ZSET_2 => {
                let mut object = RedisObject::create_zset_object();
                let len = self.rdb_load_len(buf)?;
                for _ in 0..len {
                    let ele = self.load_string(buf)?;
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
    fn rdb_load_len(&self, buf: &mut BytesMut) -> Result<u64> {
        let len_type = buf.get_u8();
        let len = match len_type {
            RDB_ENCVAL => {
                (buf[0] & 0x3F) as u64
            }
            RDB_6BITLEN => {
                buf.get_u8() as u64
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

    fn load_string(&self, buf: &mut BytesMut) -> Result<String> {
        let len = self.rdb_load_len(buf)?;
        let s = String::from_utf8(buf.split_to(len as usize).to_vec()).map_err(|_| PersistError::DecodeErr("invalid utf-8".to_string()))?;
        Ok(s)
    }
}

