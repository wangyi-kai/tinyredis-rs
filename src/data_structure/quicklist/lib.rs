pub struct QuickListLzf {
    pub sz: usize,
    pub compressed: Vec<u8>,
}

impl QuickListLzf {
    pub fn new() -> Self {
        Self {
            sz: 0,
            compressed: Vec::new(),
        }
    }

    pub fn set(&mut self, len: usize, data: Vec<u8>) {
        self.sz = len;
        self.compressed = data;
    }

    pub fn to_u8(&mut self) -> Vec<u8> {
        let data = self.sz.to_le_bytes();
        let mut buf = vec![0; self.sz + data.len()];
        buf[..data.len()].copy_from_slice(&data);
        buf[data.len()..].copy_from_slice(&self.compressed);
        buf
    }

    pub fn from_u8(data: &[u8]) -> Self {
        let len = usize::from_le_bytes(data[0..8].try_into().unwrap());
        let compress = data[8..].to_vec();

        Self {
            sz: len,
            compressed: compress,
        }
    }
}
