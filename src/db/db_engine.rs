use std::sync::{Arc};
use tokio::sync::Mutex;
use std::time::Duration;
use crate::db::db::RedisDb;
use crate::db::kvstore::KVSTORE_ALLOCATE_DICTS_ON_DEMAND;
use crate::db::object::RedisObject;
use crate::persistence::rdb::Rdb;
use crate::server::{REDIS_CONFIG, REDIS_SERVER};

#[derive(Debug)]
pub struct DbHandler {
    sender: Vec<crate::MpscSender>,
}

impl DbHandler {
    pub fn new(db_num: u32) -> Self {
        let slot_count_bits = 4;
        let flag = KVSTORE_ALLOCATE_DICTS_ON_DEMAND;
        let mut db_list = vec![];
        let mut sender_list = vec![];
        for i in 0..db_num {
            let db: RedisDb<RedisObject<String>> = RedisDb::create(slot_count_bits, flag, i as i32);
            sender_list.push(db.sender.clone());
            db_list.push(Arc::new(db));
        }
        let rdb = Arc::new(Mutex::new(Rdb::create(db_list.clone())));

        unsafe {
            let save_params = REDIS_CONFIG.get().unwrap().get_param();
            for param in save_params {
                let rdb = rdb.clone();
                tokio::spawn(async move {
                    let duration = Duration::from_secs(param.seconds);
                    loop {
                        tokio::time::sleep(duration).await;
                        let mut rdb_guard = rdb.lock().await;
                        for i in 0..db_num {
                            let _ = rdb_guard.save(i as usize).await;
                        }
                    }
                });
            }

            for db in db_list {
                tokio::spawn(async move {
                    let db_mut = Arc::into_raw(db) as *mut RedisDb<RedisObject<String>>;
                    (&mut *db_mut).run().await
                });
            }
        }

        Self {
            sender: sender_list
        }
    }

    pub fn get_sender(&self, idx: usize) -> Option<crate::MpscSender> {
        self.sender.get(idx).cloned()
    }

    pub fn get_size(&self) -> usize {
        self.sender.len()
    }
}