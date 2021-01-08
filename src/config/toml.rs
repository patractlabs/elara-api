use serde_derive::Deserialize;

use std::fs::File;
use std::io::prelude::*;

#[derive(Deserialize, Debug)]
pub struct KafkaConfig {
    pub url: String,
    pub topic: String,
    mode: String,
    username: String,
    password: String,
}

#[derive(Deserialize, Debug)]
pub struct SvrConfig {
    pub name: String,
    pub url: String,
}

#[derive(Deserialize, Debug)]
pub struct ChainConfig {
    pub name: String,
    pub rpcUrl: String,
    pub wsUrl: String,
}

#[derive(Deserialize, Debug)]
pub struct LogConfig {
    pub level: String,
}

#[derive(Deserialize, Debug)]
pub struct ApiConfig {
    pub log: LogConfig,
    pub kafka: KafkaConfig,
    pub stat: SvrConfig,
    pub ws: SvrConfig,
    pub chains: Vec<ChainConfig>,
}

pub fn ParseConfig(path: &str) -> ApiConfig {
    let mut file = File::open(path).expect("open file fail");
    let mut str_val = String::new();
    file.read_to_string(&mut str_val)
        .expect("Error Reading config file");

    let config: ApiConfig = toml::from_str(&str_val).unwrap();
    config
}
