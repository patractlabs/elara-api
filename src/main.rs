#![feature(async_closure)]
#![feature(proc_macro_hygiene, decl_macro)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_must_use)]

use clap::{App, Arg};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

mod config;
use crate::config::toml;

mod http;
use crate::http::server::{HttpServer, Broadcaster};
use crate::http::validator::Validator;

mod mq;
mod ws;
use crate::ws::relay::WSProxy;

use crossbeam_channel::bounded;

#[tokio::main]
async fn main() {
    let matches = App::new("elara api")
        .version("0.0.1")
        .author("Patract Lab")
        .about("commandline argument parsing")
        .arg(Arg::with_name("file")
                 .short("f")
                 .long("file")
                 .takes_value(true)
                 .help("A config absolute file path"))
        .get_matches();
    let configFile = matches.value_of("file").unwrap();

    let config = toml::ParseConfig(configFile);
    println!("{:?}", config);

    let mut chainsWS = HashMap::new();
    let mut chainsRPC = HashMap::new();
    for item in config.chains {
        chainsWS.insert(item.name.clone(), item.wsUrl);
        chainsRPC.insert(item.name, item.rpcUrl);
    }

    let vali = Arc::new(Mutex::new(Validator::new(config.stat.url)));

    let (tx, rx) = bounded(10);
    let achainrpc = Arc::new(chainsRPC);
    let rpcSvr = HttpServer::new(achainrpc.clone(), vali.clone(), tx);
    let notifier = Broadcaster::new(config.kafka.url, config.kafka.topic, rx);

    let achainws = Arc::new(chainsWS);
    let wsSvr = WSProxy::new(config.ws.url, achainws.clone(), vali.clone());

    notifier.Start();
    rpcSvr.Start();
    wsSvr.Start();
}