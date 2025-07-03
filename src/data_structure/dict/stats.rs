use crate::data_structure::dict::dict::Dict;
use crate::data_structure::dict::lib::{dict_size, DICT_STATS_VECTLEN};
use std::fmt::Display;
use std::hash::Hash;

use std::fmt::Write as _;

pub struct DictStats {
    ht_idx: usize,
    buckets: u64,
    max_chain_len: u64,
    total_chain_len: u64,
    ht_size: u64,
    ht_used: u64,
    cl_vector: Vec<u64>,
}

pub fn dict_combine_stats(from: &mut DictStats, into: &mut DictStats) {
    into.buckets += from.buckets;
    into.max_chain_len = from.max_chain_len.max(into.max_chain_len);
    into.total_chain_len += from.total_chain_len;
    into.ht_size += from.ht_size;
    into.ht_used += from.ht_used;
    for i in 0..DICT_STATS_VECTLEN {
        into.cl_vector[i] += from.cl_vector[i];
    }
}

pub fn dict_get_stats_msg(
    buf: &mut String,
    buf_size: usize,
    stats: &DictStats,
    full: bool,
) -> usize {
    if stats.ht_used == 0 {
        let table_type = if stats.ht_idx == 0 {
            "main hash table"
        } else {
            "rehashing target"
        };
        write!(
            buf,
            "Hash table {} stats ({}): No stats available for empty dictionaries\n",
            stats.ht_idx, table_type
        )
        .unwrap();
        return buf.len();
    }
    let table_type = if stats.ht_idx == 0 {
        "main hash table"
    } else {
        "rehashing target"
    };
    let mut l = 0;
    write!(buf, "Hash table {} stats ({}): No stats available for empty dictionaries\n, table size: {}\n number of elements: {}\n", stats.ht_idx, table_type, stats.ht_size, stats.ht_used).unwrap();
    l += buf.len();
    if full {
        let before_len = buf.len();
        write!(&mut buf[l..].to_string(), "different slots: {}, max chain length: {}\n avg chain length (counted): {}, avg chain length (computed): {}\n Chain length distribution: \n", stats.buckets, stats.max_chain_len, stats.total_chain_len / stats.buckets, stats.ht_used / stats.buckets).unwrap();
        l += buf.len() - before_len;

        for i in 0..DICT_STATS_VECTLEN - 1 {
            if stats.cl_vector[i] == 0 {
                continue;
            }
            if l >= buf_size {
                break;
            }

            let before_len = buf.len();
            write!(
                &mut buf[l..].to_string(),
                " {}: {}({})\n",
                i,
                stats.cl_vector[i],
                stats.cl_vector[i] / stats.ht_size + 1
            )
            .unwrap();
            l += buf.len() - before_len;
        }
    }
    buf.len()
}

impl<V> Dict<V>
where V: Default + PartialEq + Clone,
{
    pub fn get_stats_ht(&self, ht_idx: usize, full: bool) -> DictStats {
        let cl_vector = Vec::with_capacity(DICT_STATS_VECTLEN);
        let mut stats = DictStats {
            ht_idx,
            buckets: 0,
            max_chain_len: 0,
            cl_vector,
            ht_size: dict_size(self.ht_size_exp[ht_idx]),
            ht_used: self.ht_used[ht_idx] as u64,
            total_chain_len: 0,
        };
        if !full {
            return stats;
        }
        unsafe {
            for i in 0..dict_size(self.ht_size_exp[ht_idx]) {
                if self.ht_table[ht_idx][i as usize].is_none() {
                    stats.cl_vector[0] += 1;
                    continue;
                }
                stats.buckets += 1;
                let mut chain_len = 0;
                let mut he = self.ht_table[ht_idx][i as usize];
                while he.is_some() {
                    chain_len += 1;
                    he = (*he.unwrap().as_ptr()).next;
                }
                if chain_len < DICT_STATS_VECTLEN {
                    stats.cl_vector[chain_len] += 1;
                } else {
                    stats.cl_vector[DICT_STATS_VECTLEN - 1] += 1;
                }
                stats.total_chain_len += chain_len as u64;
            }
        }
        stats
    }

    pub fn get_stats(&self, buf: &mut String, mut buf_size: usize, full: bool) {
        let mut l = 0;
        let origin_buf = buf.clone();
        let origin_buf_size = buf_size;

        let mut main_ht_stats = self.get_stats_ht(0, full);
        l = dict_get_stats_msg(buf, buf_size, &mut main_ht_stats, full);
        buf_size -= l;

        if self.dict_is_rehashing() && buf_size > 0 {
            let mut rehash_stats = self.get_stats_ht(1, full);
            dict_get_stats_msg(buf, buf_size, &mut rehash_stats, full);
        }
    }
}
