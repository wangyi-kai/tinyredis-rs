use std::{
    io::SeekFrom,
    sync::Arc
};
use std::sync::Mutex;
use bytes::{BufMut, BytesMut};
use tokio::io::{AsyncSeekExt, AsyncWriteExt, BufWriter};
use crate::db::db::RedisDb;
use crate::persistence::error::PersistError;
use crate::db::object::{*};
use crate::persistence::{*};

#[derive(Debug, Clone)]
pub struct SaveParam {
    pub seconds: u64,
    pub changes: usize,
}

pub struct Rdb {
    file: tokio::fs::File,
    db: Arc<Mutex<RedisDb<RedisObject<String>>>>,
    buf: BytesMut,
}

impl Rdb {
    pub async fn create(db: Arc<Mutex<RedisDb<RedisObject<String>>>>) -> Result<Self, PersistError> {
        let file_path = "dump.rdb".to_string();
        let rdb_file = tokio::fs::File::create(file_path).await?;

        Ok(Self {
            file: rdb_file,
            db,
            buf: BytesMut::with_capacity(1024 * 8),
        })
    }

    pub async fn save_db(&mut self, db_id: usize) -> Result<(), PersistError> {
        self.file.seek(SeekFrom::Start(0)).await
            .map_err(|_| PersistError::FileError(-2))?;
        self.buf.extend_from_slice(b"RDB");
        self.buf.put_u8(RDB_OPCODE_SELECTDB);

        let kvs_it = self.db.lock().unwrap().kvs.iter();
        for mut dict in kvs_it {
            let key = dict.get_key();
            let value = dict.value();
            self.rdb_save_key_value_pair(key, value)?;
        }
        self.file.write_all(&self.buf).await?;

        Ok(())
    }

    #[inline(always)]
    fn rdb_save_object_type(&mut self, object: &RedisObject<String>) -> Result<(), PersistError> {
        match object.encoding {
            OBJ_STRING => {
                self.buf.put_u8(RDB_TYPE_STRING);
            }
            OBJ_SET => {
                match object.encoding {
                    OBJ_ENCODING_INTSET => self.buf.put_u8(RDB_TYPE_SET_INTSET),
                    OBJ_ENCODING_HT => self.buf.put_u8(RDB_TYPE_SET),
                    _ => return Err(PersistError::EncodingErr("Unknown set encoding".to_string()))
                }
            }
            OBJ_ZSET => {
                match object.encoding {
                    OBJ_ENCODING_SKIPLIST => self.buf.put_u8(RDB_TYPE_ZSET_2),
                    _ => return Err(PersistError::EncodingErr("Unknown sorted set encoding".to_string()))
                }
            }
            OBJ_HASH => {
                match object.encoding {
                    OBJ_ENCODING_HT => self.buf.put_u8(RDB_TYPE_HASH),
                    _ => return Err(PersistError::EncodingErr("Unknown hash encoding".to_string()))
                }
            }
            _ => {
                return Err(PersistError::EncodingErr("Unknown object type".to_string()))
            }
        }
        Ok(())
    }

    fn rdb_save_string(&mut self, s: &str) -> Result<usize, PersistError> {
        let s_vec = s.as_bytes();
        let len = s_vec.len();
        self.rdb_save_len(len as u64)?;
        self.buf.put_slice(s_vec);
        Ok(len)
    }


    #[inline(always)]
    fn rdb_save_len(&mut self, len: u64) -> Result<usize, PersistError> {
        let mut buf = [0u8; 2];
        let mut nwritten = 0;
        if len < 1 << 6 {
            buf[0] = (len as u8) | (RDB_6BITLEN << 6);
            self.buf.extend_from_slice(&buf);
            nwritten = 1;
        } else if len < 1 << 14 {
            buf[0] = ((len >> 8) as u8) | (RDB_14BITLEN << 6);
            buf[1] = (len & 0xFF) as u8;
            nwritten = 2;
        } else if len <= u32::MAX as u64 {
            buf[0] = RDB_32BITLEN;
            self.buf.extend_from_slice(&buf);
            let len32 = (len as u32).to_be_bytes();
            self.buf.extend_from_slice(&len32);
            nwritten = 1 + 4;
        } else {
            buf[0] = RDB_6BITLEN;
            self.buf.extend_from_slice(&buf);
            let len64 = len.to_be_bytes();
            self.buf.extend_from_slice(&len64);
            nwritten = 1 + 8;
        }
        Ok(nwritten)
    }

    fn rdb_save_double_value(&mut self, val: f64) -> Result<usize, PersistError> {
        let f = val.to_ne_bytes();
        let len = f.len();
        self.buf.put_slice(&f);
        Ok(len)
    }

    fn rdb_save_key_value_pair(&mut self, key: &str, value: &RedisObject<String>) -> Result<(), PersistError> {
        self.rdb_save_object_type(value)?;
        self.rdb_save_string(key)?;
        self.rdb_save_object(value)?;
        Ok(())
    }

    fn rdb_save_object(&mut self, object: &RedisObject<String>) -> Result<(), PersistError> {
        let mut nwritten = 0;
        match object.object_type {
            OBJ_STRING => {
                match &object.ptr {
                    RedisValue::String(s) => {
                        let n = self.rdb_save_string(s)?;
                        nwritten += n;
                    }
                    _ => {
                        return Err(PersistError::TypeErr("err object type, expect string".to_string()))
                    }
                }
            }
            OBJ_HASH => {
                match &object.ptr {
                    RedisValue::Hash(ht) => {
                        let ht_iter = ht.iter();
                        let size = ht.dict_size();
                        nwritten += self.rdb_save_len(size as u64)?;
                        for entry in ht_iter {
                            let field = entry.get_key();
                            let value = entry.value();
                            nwritten += self.rdb_save_string(field)?;
                            nwritten += self.rdb_save_string(value)?;
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
                        return Err(PersistError::TypeErr("err object type, expect zset".to_string()))
                    }
                }
            }
            _ => {

            }
        }
        Ok(())
    }

    #[inline(always)]
    fn rdb_load_len(&mut self, buf: &mut BytesMut) -> Result<u64, PersistError> {
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
                let mut b = [0u8; 4];
                b.copy_from_slice(buf.get(0..4).unwrap());
                u32::from_be_bytes(b) as u64
            }
            RDB_64BITLEN => {
                let mut b = [0u8; 8];
                b.copy_from_slice(buf.get(0..8).unwrap());
                u64::from_be_bytes(b)
            }
            _ => {
                return Err(PersistError::LoadErr(format!("Unknown length encoding {} in rdb_load_len", len_type)));
            }
        };

        Ok(len)
    }
}

