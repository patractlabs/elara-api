use serde::Deserialize;

use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::prelude::*;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub log: LogConfig,
    pub kafka: HashMap<String, String>,
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

pub fn load_config(path: &str) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut res = String::new();
    let _ = file.read_to_string(&mut res)?;
    Ok(res)
}
