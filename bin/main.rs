use clap::{App, Arg};
use log::{info, warn};
use rdkafka::client::ClientContext;
use rdkafka::config::{ClientConfig, RDKafkaLogLevel};
use rdkafka::consumer::stream_consumer::StreamConsumer;
use rdkafka::consumer::{CommitMode, Consumer, ConsumerContext, Rebalance};
use rdkafka::error::KafkaResult;
use rdkafka::message::{Headers};
use rdkafka::topic_partition_list::TopicPartitionList;
use rdkafka::util::get_rdkafka_version;

use elara_api::websocket3;

use elara_api::kafka::{KvConsumer, LogLevel};
use elara_api::websocket3::{WsServer, WsConnection};
use futures::{StreamExt, SinkExt};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio::sync::mpsc::Receiver;
use tokio::time::Duration;

use tokio_tungstenite::{accept_async, tungstenite};
use tungstenite::{Error, Message};
use log::*;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let server = WsServer::bind("localhost:9002").await?;

    while let Ok(connection) = server.accept().await {
        tokio::spawn( accept_connection(connection));
    }

    Ok(())
}


    async fn accept_connection(connection: WsConnection) {
        if let Err(e) = handle_connection(connection).await {
            match e {
                Error::ConnectionClosed | Error::Protocol(_) | Error::Utf8 => (),
                err => error!("Error processing connection: {}", err),
            }
        }
    }

    async fn handle_connection(connection: WsConnection) -> tungstenite::Result<()> {
        info!("New WebSocket connection: {}", connection.addr);
        let (mut ws_sender, mut ws_receiver) = connection.stream.split();
        let mut interval = tokio::time::interval(Duration::from_millis(1000));

        // Echo incoming WebSocket messages and send a message periodically every second.
        loop {
            tokio::select! {
                msg = ws_receiver.next() => {
                    match msg {
                        Some(msg) => {
                            let msg = msg?;
                            if msg.is_text() ||msg.is_binary() {
                                ws_sender.send(msg).await?;
                            } else if msg.is_close() {
                                break;
                            }
                        }
                        None => break,
                    }
                }
                _ = interval.tick() => {
                    ws_sender.send(Message::Text("tick".to_owned())).await?;
                }
            }
        }

        ws_sender.close().await?;

        Ok(())
    }
