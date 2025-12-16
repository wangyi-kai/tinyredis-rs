use std::{io::SeekFrom};
use tokio::{fs::File};
use bytes::{Buf, BufMut, BytesMut};
use std::sync::{Arc};
use tokio::io::{AsyncSeekExt, AsyncWriteExt};
use tokio::sync::mpsc::Sender;
use crate::db::db_engine::DbCommand;
use crate::persistence::error::PersistError;
use crate::db::object::{*};
use crate::persistence::{*};
use crate::Result;

#[derive(Clone)]
pub struct Rdb<V> {
    db_sender: Vec<Sender<DbCommand<V>>>,
    buf: BytesMut,
}

impl<V> Rdb<V> {
    pub fn create(db_sender: Vec<Sender<DbCommand<V>>>) -> Self {
        let buf = BytesMut::with_capacity(1024 * 8);
        Self { db_sender, buf }
    }

    pub async fn save(&mut self, db_id: u32) -> Result<()> {
        let tmp_path = "tmp.rdb".to_string();
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
                        let v = &*(value as *mut V as *mut RedisObject<String>);
                        self.rdb_save_key_value_pair(key, v)?;
                    }
                }
                _ => {

                }
            }
        }
        tmp_file.write_all(&self.buf).await?;
        let rdb_path = "dump.rdb".to_string();
        tokio::fs::rename(&tmp_path, &rdb_path).await?;

        Ok(())
    }

    #[inline(always)]
    fn rdb_save_object_type(&mut self, object: &RedisObject<String>) -> Result<()> {
        match object.encoding {
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

    fn rdb_save_double_value(&mut self, val: f64) -> Result<usize> {
        let f = val.to_ne_bytes();
        let len = f.len();
        self.buf.put_slice(&f);
        Ok(len)
    }

    fn rdb_save_key_value_pair(&mut self, key: &str, value: &RedisObject<String>) -> Result<()> {
        self.rdb_save_object_type(value)?;
        self.rdb_save_string(key)?;
        self.rdb_save_object(value)?;
        Ok(())
    }

    fn rdb_save_object(&mut self, object: &RedisObject<String>) -> Result<()> {
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
                                let value = (*entry).value();
                                nwritten += self.rdb_save_string(field)?;
                                nwritten += self.rdb_save_string(value)?;
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
                                nwritten += self.rdb_save_string(&(*node.as_ptr()).get_elem())?;
                                nwritten += self.rdb_save_double_value((*node.as_ptr()).get_score())?;
                                zn = (*node.as_ptr()).back_ward();
                            }
                        }
                    }
                    _ => {
                        return Err(PersistError::TypeErr("err object type, expect zset".to_string()).into())
                    }
                }
            }
            _ => {

            }
        }
        Ok(())
    }

    #[inline(always)]
    fn rdb_load_len(&mut self, buf: &mut BytesMut) -> Result<u64> {
        let len_type = (buf[0] & 0xC0) >> 6;
        let len = match len_type {
            RDB_ENCVAL => {
                (buf[0] & 0x3F) as u64
            }
            RDB_6BITLEN => {
                (buf[0] & 0x3F) as u64
            }
            RDB_14BITLEN => {
                ((buf[0] as u64 & 0x3F) << 8) | buf[1] as u64
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

    fn load_string(&mut self, buf: &mut BytesMut) -> Result<()> {
        let rdb_flag = String::from_utf8(buf[0..3].to_vec())?;
        if rdb_flag.ne("RDB") {
            return Err(PersistError::DecodeErr("flag is not RDB".to_string()).into());
        }
        buf.advance(3);
        let select_code = buf.get_u8();

        Ok(())
    }
}

