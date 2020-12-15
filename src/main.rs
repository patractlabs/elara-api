#![feature(async_closure)]
#![feature(proc_macro_hygiene, decl_macro)]
#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_must_use)]

use clap::{App, Arg};
use std::collections::HashMap;
use std::sync::{Arc};

use log::*;
use simplelog::*;

mod config;
use crate::config::toml;

mod http;
use crate::http::server::{Broadcaster};
use crate::http::validator::Validator;
use crate::http::actix_server::{ActixWebServer, RunServer};

mod mq;
mod ws;
use crate::ws::relay::{WSProxy, RunWebSocketBg};

use crossbeam_channel::bounded;
use crate::http::server::MessageSender;

fn main() {
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

    LogInit(&config.log);
    info!("{:?}", config);

    let mut chainsWS = HashMap::new();
    let mut chainsRPC = HashMap::new();
    for item in config.chains {
        chainsWS.insert(item.name.clone(), item.wsUrl);
        chainsRPC.insert(item.name, item.rpcUrl);
    }

    let vali = Validator::new(config.stat.url);

    let (tx, rx) = bounded(10);
    let achainrpc = Arc::new(chainsRPC);
    // rocket server
    // let rpcSvr = HttpServer::new(achainrpc.clone(), vali.clone(), MessageSender::<(String, String)>::new(tx.clone()));

    //actix server
    let rpcSvr = ActixWebServer::new(achainrpc.clone(), vali.clone(), MessageSender::<(String, String)>::new(tx.clone()));

    let notifier = Broadcaster::new(config.kafka.url, config.kafka.topic, rx);

    let achainws = Arc::new(chainsWS);
    let wsSvr = WSProxy::new(config.ws.url, achainws.clone(), vali.clone(), MessageSender::<(String, String)>::new(tx.clone()));

    notifier.Start();
    // rpcSvr.Start();
    // wsSvr.Start().await;
    RunWebSocketBg(wsSvr);
    RunServer(rpcSvr, config.http.port);
}

fn LogInit(cfg: &toml::LogConfig) {
    let level = match &cfg.level[..] {
        "Warn" => LevelFilter::Warn,
        "Error" => LevelFilter::Error,
        "Info" => LevelFilter::Info,
        "Debug" => LevelFilter::Debug,
        "Trace" => LevelFilter::Trace,
        _ => LevelFilter::Off
    };
    
    CombinedLogger::init(
        vec![
            TermLogger::new(level, Config::default(), TerminalMode::Mixed), //terminal logger
            // WriteLogger::new(LogLevelFilter::Info, Config::default(), File::create("my_rust_binary.log").unwrap()) //记录日志到"*.log"文件中
        ]
    ).unwrap();
}