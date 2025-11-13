use std::fs::File;
use std::io::Read;
use json_comments::StripComments;
use serde::{Deserialize, Serialize};
use crate::client::config::{ClientConfig, CONFIG_PATH_JSON, CONFIG_PATH_TOML};
use crate::persistence::rdb_config::RdbConfig;

pub const SERVER_CONFIG_JSON: &str = "./server_config.json";
pub const SERVER_CONFIG_TOML: &str = "./server_config.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub db_num: u32,
    pub hash_max_ziplist_entries: usize,
    pub hash_max_ziplist_value: usize,
    pub rdb_config: RdbConfig,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            db_num: 1,
            hash_max_ziplist_entries: 512,
            hash_max_ziplist_value: 64,
            rdb_config: RdbConfig::default(),
        }
    }
}

impl ServerConfig {
    pub fn new(path: Option<&str>) -> Self {
        let config_path_show;
        let mut file = if let Some(path) = path {
            config_path_show = path;
            if let Ok(file) = File::open(&path) {
                file
            } else {
                println!("Config File: {} Read Fail, Use Default Config. ", config_path_show);
                return ServerConfig::default();
            }
        } else {
            if let Ok(file) = File::open(SERVER_CONFIG_JSON) {
                config_path_show = SERVER_CONFIG_JSON;
                file
            } else {
                if let Ok(file) = File::open(SERVER_CONFIG_TOML) {
                    config_path_show = SERVER_CONFIG_TOML;
                    file
                } else {
                    println!("Config File: {} Read Fail, Use Default Config. ", SERVER_CONFIG_JSON);
                    return ServerConfig::default();
                }
            }
        };
        let mut config_string = String::new();
        if let Err(e) = file.read_to_string(&mut config_string) {
            println!("Config File: {} Read Fail{e}, Use Default Config. ", config_path_show);
            return ServerConfig::default();
        }
        println!("Config File: {}", config_path_show);
        if let Ok(config) = toml::from_str(&config_string) {
            config
        } else {
            let config_string = StripComments::new(config_string.as_bytes());
            match serde_json::from_reader(config_string) {
                Ok(data) => data,
                Err(e) => {
                    println!("Config File: {} Read Fail{e}, Use Default Config.", config_path_show);
                    ServerConfig::default()
                }
            }
        }
    }

    pub fn set_rdb_save_param(&mut self, seconds: u64, changes: usize) {
        self.rdb_config.set_save_params(seconds, changes);
    }
}