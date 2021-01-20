use serde::Deserialize;

use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::prelude::*;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub log: LogConfig,
    pub kafka: KafkaConfig,
    pub ws: WsConfig,
}

#[derive(Deserialize, Debug, Clone)]
pub struct WsConfig {
    pub addr: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct KafkaConfig {
    pub topics: Vec<String>,
    pub config: HashMap<String, String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct LogConfig {
    pub level: String,
}

pub fn load_config(path: &str) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut res = String::new();
    let _ = file.read_to_string(&mut res)?;
    Ok(res)
}
