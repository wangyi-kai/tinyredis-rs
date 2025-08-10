use bytes::Bytes;
use crate::db::db::RedisDb;
use crate::db::object::{OBJ_ENCODING_SKIPLIST, RedisObject, RedisValue};
use crate::parser::cmd::command::{CommandStrategy, RedisCommand};
use crate::parser::cmd::error::CommandError;
use crate::parser::frame::Frame;

#[allow(dead_code)]
#[derive(Debug)]
pub enum SortedCmd {
    /// Adds one or more members to a sorted set, or updates their scores.
    ZAdd {key: String, arg: Option<String>, values: Vec<String>},
    /// Returns the number of members in a sorted set
    ZCard {key: String},
    /// Returns the score of a member in a sorted set
    ZScore {key: String, member: String},
    /// Returns the union of multiple sorted sets
    ZUnion,
    /// Returns the intersect of multiple sorted sets
    ZInter,
    /// Returns the number of members of the intersect of multiple sorted sets
    ZInterCard,
    /// Stores the intersect of multiple sorted sets in a key
    ZInterStore,
}

impl CommandStrategy for SortedCmd {
    fn into_frame(self) -> Frame {
        let mut frame = Frame::Array(vec![]);
        match self {
            SortedCmd::ZAdd {key, arg, values} => {
                frame.push_bulk(Bytes::from("zadd".as_bytes()));
                frame.push_bulk(Bytes::from(key.into_bytes()));
                if let Some(arg) = arg {
                    frame.push_bulk(Bytes::from(arg.into_bytes()));
                } else {
                    frame.push_bulk(Bytes::from("null".as_bytes()));
                }
                for value in values {
                    frame.push_bulk(Bytes::from(value.into_bytes()));
                }
                frame
            }
            SortedCmd::ZCard { key} => {
                frame.push_bulk(Bytes::from("zcard".as_bytes()));
                frame.push_bulk(Bytes::from(key.into_bytes()));
                frame
            }
            SortedCmd::ZScore { key, member } => {
                frame.push_bulk(Bytes::from("zscore".as_bytes()));
                frame.push_bulk(Bytes::from(key.into_bytes()));
                frame.push_bulk(Bytes::from(member.into_bytes()));
                frame
            }
            _ => Frame::Null,
        }
    }

    fn from_frame(name: &str, frame: Frame) -> crate::Result<RedisCommand> {
        match name {
            "zadd" => {
                let len = frame.get_len();
                let key = frame.get_frame_by_index(1).ok_or("command error 'zadd'")?.to_string();
                let mut arg = frame.get_frame_by_index(2).ok_or("command error 'zadd'")?.to_string();
                let arg = if arg.eq("null") {
                    None
                } else {
                    Some(arg)
                };
                let mut members = Vec::with_capacity(len);
                for i in 3..len {
                    let member = frame.get_frame_by_index(i).ok_or("command error 'zadd'")?.to_string();
                    members.push(member);
                }
                Ok(RedisCommand::SortSet(SortedCmd::ZAdd {key, arg, values: members}))
            }
            "zcard" => {
                let key = frame.get_frame_by_index(1).ok_or("command error 'zcard'")?.to_string();
                Ok(RedisCommand::SortSet(SortedCmd::ZCard {key}))
            }
            "zscore" => {
                let key = frame.get_frame_by_index(1).ok_or("command error 'zscore'")?.to_string();
                let member = frame.get_frame_by_index(2).ok_or("command error 'zscore'")?.to_string();
                Ok(RedisCommand::SortSet(SortedCmd::ZScore {key, member}))
            }
            _ => Err(CommandError::ParseError(-1).into())
        }
    }

    fn apply(self, db: &mut RedisDb<RedisObject<String>>) -> crate::Result<Frame> {
        match self {
            SortedCmd::ZAdd { key, arg, values} => {
                let key = RedisObject::<String>::create_string_object(key);
                let o = db.find(&key);
                let len = values.len();
                if let Some(o) = o {
                    for i in (0..len - 1).step_by(2) {
                        let score: f64 = values[i].clone().parse()?;
                        let ele = values[i + 1].clone();
                        Self::zadd(o, arg.clone(), score, ele);
                    }
                } else {
                    let mut z_obj = RedisObject::<String>::create_zset_object();
                    for i in (0..len - 1).step_by(2) {
                        let score: f64 = values[i].clone().parse()?;
                        let ele = values[i + 1].clone();
                        Self::zadd(&mut z_obj, arg.clone(), score, ele);
                    }
                    let o = db.add(key, z_obj);
                    if o.is_none() {
                        return Err(CommandError::ExecuteFail("zadd".to_string()).into());
                    }
                }
                Ok(Frame::Simple((len >> 1).to_string()))
            }
            SortedCmd::ZCard {key} => {
                let key = RedisObject::<String>::create_string_object(key);
                let o = db.find(&key);
                if let Some(o) = o {
                    if o.encoding == OBJ_ENCODING_SKIPLIST {
                        match &o.ptr {
                            RedisValue::SortSet(zset) => Ok(Frame::Simple(zset.zsl.length.to_string())),
                            _ => Ok(Frame::Null),
                        }
                    } else {
                        Ok(Frame::Null)
                    }
                } else {
                   Ok(Frame::Null)
                }
            }
            SortedCmd::ZScore {key, member} => {
                let key = RedisObject::<String>::create_string_object(key);
                let o = db.find(&key);
                if let Some(o) = o {
                    match &mut o.ptr {
                        RedisValue::SortSet(zset) => {
                            let de = zset.dict.find(&member);
                            if let Some(de) = de {
                                unsafe {
                                    let score = *(*de.as_ptr()).value();
                                    Ok(Frame::Simple(score.to_string()))
                                }
                            } else {
                                Err(CommandError::NotExist(format!("{} not exist", member)).into())
                            }
                        }
                        _ => Ok(Frame::Null)
                    }
                } else {
                    Ok(Frame::Null)
                }
            }
            _ => todo!()
        }
    }
}

impl SortedCmd {
    pub fn zadd(o: &mut RedisObject<String>, arg: Option<String>, mut score: f64, ele: String) {
        if o.encoding == OBJ_ENCODING_SKIPLIST {
            let zs = match &mut o.ptr {
                RedisValue::SortSet(zset) => zset,
                _ => return,
            };
            let de = zs.dict.find(&ele);
            let arg = if let Some(arg) = arg {
                arg
            } else {
                "".to_string()
            };
            if let Some(de) = de {
                unsafe {
                    let cur_score = (*de.as_ptr()).value();
                    if arg.eq("nx") {
                        return;
                    }
                    if arg.eq("incr") {
                        score += cur_score;
                    }
                    if (arg.eq("lt") && score >= *cur_score) || (arg.eq("gt") && score <= *cur_score) {
                        return;
                    }
                    if score != *cur_score {
                        let node = zs.zsl.update_score(*cur_score, &ele, score);
                        (*de.as_ptr()).val = Some((*node.as_ptr()).get_score());
                    }
                }
            } else {
                unsafe {
                    let node = zs.zsl.insert(score, ele.clone());
                    let score = (*node.as_ptr()).get_score();
                    if let Err(e) = zs.dict.add_raw(ele, score) {
                        println!("zadd err: {e}");
                        return;
                    }
                }
            }
        }
    }
}