use std::fs::File;
use std::io::{Read};
use serde::{Deserialize, Serialize};
use serde_json::{Value};
use json_comments::StripComments;


pub const CONFIG_PATH_TOML: &str = "./config.toml";
pub const CONFIG_PATH_JSON: &str = "./config.json";

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Config {
    #[serde(default = "server_ip")]
    server_ip: String,
    #[serde(default = "server_port")]
    server_port: u16,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_port: server_port(),
            server_ip: server_ip(),
        }
    }
}

impl Config {
    pub fn new(config_path: Option<&str>) -> Self {
        let mut config_path_show;
        let mut file = if let Some(path) = config_path {
            config_path_show = path;
            if let Ok(file) = File::open(&path) {
                file
            } else {
                println!("Config File: {} Read Fail, Use Default Config. ", config_path_show);
                return Config::default();
            }
        } else {
            if let Ok(file) = File::open(CONFIG_PATH_JSON) {
                config_path_show = CONFIG_PATH_JSON;
                file
            } else {
                if let Ok(file) = File::open(CONFIG_PATH_TOML) {
                    config_path_show = CONFIG_PATH_TOML;
                    file
                } else {
                    println!("Config File: {} Read Fail, Use Default Config. ", CONFIG_PATH_JSON);
                    return Config::default();
                }
            }
        };
        let mut config_string = String::new();
        if let Err(e) = file.read_to_string(&mut config_string) {
            println!("Config File: {} Read Fail{e}, Use Default Config. ", config_path_show);
            return Config::default();
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
                    Config::default()
                }
            }
        }
    }

    pub fn get_value(&self, k: &str) -> Option<String> {
        let str = serde_json::to_string(&self).unwrap();
        let mut value = serde_json::from_str::<Value>(&str).unwrap();
        let mut sub_value = &mut value;
        for k in k.split(".") {
            sub_value = if let Some(v) = sub_value.get_mut(k) { v } else { return None; }
        }
        Some(sub_value.to_string())
    }
}

fn server_ip() -> String { "127.0.0.1".to_string() }

fn server_port() -> u16 { 8000 }