use serde_derive::Deserialize;

use std::fs::File;
use std::io::prelude::*;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub log: LogConfig,
    pub kafka: KafkaConfig,
    pub stat: ServerConfig,
    pub ws: ServerConfig,
    pub chains: Vec<ChainConfig>,
}

#[derive(Deserialize, Debug)]
pub struct KafkaConfig {
    pub url: String,
    pub topic: String,
    pub mode: String,
    pub username: String,
    pub password: String,
}

#[derive(Deserialize, Debug)]
pub struct ServerConfig {
    pub name: String,
    pub url: String,
}

#[derive(Deserialize, Debug)]
pub struct ChainConfig {
    pub name: String,
    pub rpc_url: String,
    pub ws_url: String,
}

#[derive(Deserialize, Debug)]
pub struct LogConfig {
    pub level: String,
}

pub fn load_config(path: &str) -> Config {
    let mut file = File::open(path).expect("open file fail");
    let mut str_val = String::new();
    file.read_to_string(&mut str_val)
        .expect("Error Reading config file");

    let config: Config = toml::from_str(&str_val).unwrap();
    config
}
