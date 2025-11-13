use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RdbConfig {
    save_param: Vec<SaveParam>,
    save_param_len: u32,
    rdb_file_name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SaveParam {
    pub seconds: u64,
    pub changes: usize,
}

impl Default for RdbConfig {
    fn default() -> Self {
        Self {
            save_param: Vec::new(),
            save_param_len: 1,
            rdb_file_name: "dump.rdb".to_string(),
        }
    }
}

impl RdbConfig {
    pub fn set_save_params(&mut self, seconds: u64, changes: usize) {
        let save_param = SaveParam {seconds, changes};
        self.save_param.push(save_param);
    }
}