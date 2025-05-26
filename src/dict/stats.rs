use crate::dict::lib::DICT_STATS_VECTLEN;

pub struct DictStats {
    ht_idx: usize,
    buckets: u64,
    max_chain_len: u64,
    total_chain_len: u64,
    ht_size: u64,
    ht_used: u64,
    cl_vector: Vec<u64>,
}


pub fn dict_get_stats_msg(mut buf: String, buf_size: usize, stats: &DictStats, full: bool) -> usize {
    if stats.ht_used == 0 {
        let table_type = if stats.ht_idx == 0 {
            "main hash table"
        } else {
            "rehashing target"
        };
        write!(&mut buf, "Hash table {} stats ({}): No stats available for empty dictionaries\n", stats.ht_idx, table_type).unwrap();
        return buf.len()
    }
    let table_type = if stats.ht_idx == 0 {
            "main hash table"
        } else {
            "rehashing target"
        };
    let mut l = 0;
    write!(&mut buf, "Hash table {} stats ({}): No stats available for empty dictionaries\n, table size: {}\n number of elements: {}\n", stats.ht_idx, table_type, stats.ht_size, stats.ht_used).unwrap();
    l += buf.len();
    if full {
        let before_len = buf.len();
        write!(&mut buf[l..], "different slots: {}, max chain length: {}\n avg chain length (counted): {}, avg chain length (computed): {}\n Chain length distribution: \n", stats.buckets, stats.max_chain_len, stats.total_chain_len / stats.buckets, stats.ht_used / stats.buckets).unwrap();
        l += buf.len() - before_len;

        for i in 0..DICT_STATS_VECTLEN - 1 {
            if stats.cl_vector[i] == 0 {
                continue;
            }
            if l >= buf_size {
                break;
            }

            let before_len = buf.len();
            write!(&mut buf[l..], " {}: {}({})\n", i, stats.cl_vector[i], stats.cl_vector[i] / stats.ht_size + 1).unwrap();
            l += buf.len() - before_len;
        }
    }
    buf.len()
}