use log::*;
use rdkafka::client::ClientContext;
use rdkafka::config::{ClientConfig, RDKafkaLogLevel};
use rdkafka::consumer::stream_consumer::StreamConsumer;
use rdkafka::consumer::{CommitMode, Consumer, ConsumerContext, Rebalance};
use rdkafka::error::KafkaResult;
use rdkafka::message::Headers;
use rdkafka::topic_partition_list::TopicPartitionList;
use rdkafka::util::get_rdkafka_version;

use elara_api::websocket3;

use elara_api::kafka::{KVSubscriber, KvConsumer, LogLevel, OwnedMessage};
use elara_api::websocket3::{WsConnection, WsServer};
use futures::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::broadcast;
use tokio::sync::broadcast::{Receiver, Sender};
use tokio::time::Duration;

use log::*;
use tokio_tungstenite::{accept_async, tungstenite, WebSocketStream};
use tungstenite::{Error, Message};

use elara_api::config::*;
use elara_api::error::Result;
use futures::lock::Mutex;
use tokio::time;

#[actix_web::main]
async fn main() -> Result<()> {
    env_logger::init();

    let cfg = load_config("bin/config.toml").expect("need a config");
    let cfg: Config = toml::from_str(&*cfg).expect("Config is illegal");

    debug!("load config: {:#?}", &cfg);
    let consumer = KvConsumer::new(cfg.kafka.into_iter(), LogLevel::Debug);
    info!("Subscribing kafka topic `{}`", "polkadot");
    consumer.subscribe(&["polkadot"])?;

    let subscriber = KVSubscriber::new(Arc::new(consumer), 100);
    subscriber.start();
    let mut receiver = subscriber.subscribe();
    // TODO:
    // info!("{:?}", receiver.recv().await);

    let addr = "localhost:9002";
    let server = WsServer::bind(addr)
        .await
        .expect(&*format!("Cannot listen {}", addr));
    info!("Started ws server at {}", addr);
    while let Ok((stream, connection)) = server.accept().await {
        let receiver = subscriber.subscribe();
        tokio::spawn(accept_connection(stream, connection, receiver));
    }

    Ok(())
}

async fn accept_connection(
    stream: WebSocketStream<TcpStream>,
    connection: WsConnection,
    kafka_receiver: Receiver<OwnedMessage>,
) {
    if let Err(e) = handle_connection(stream, connection, kafka_receiver).await {
        match e {
            Error::ConnectionClosed | Error::Protocol(_) | Error::Utf8 => (),
            err => error!("Error processing connection: {}", err),
        }
    }
}

async fn handle_connection(
    stream: WebSocketStream<TcpStream>,
    connection: WsConnection,
    mut kafka_receiver: Receiver<OwnedMessage>,
) -> tungstenite::Result<()> {
    info!("New WebSocket connection: {}", connection.addr);
    let (mut sender, mut receiver) = stream.split();
    let mut interval = tokio::time::interval(Duration::from_millis(1000));

    let sender = Arc::new(Mutex::new(sender));
    let mut bg_sender = sender.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            let res = bg_sender
                .lock()
                .await
                .send(Message::Text("test".to_string())).await;

            match res {
                Err (Error::ConnectionClosed) => {
                    return
                },

                Err(err) => {
                    warn!("Error occurred when send subscription data: {}", err);
                    return
                }
                Ok(()) => {},
            }
        };

    });

    loop {
        tokio::select! {
            msg = receiver.next() => {
                match msg {
                    Some(msg) => {
                        let msg = msg?;
                        let mut sender = sender.lock().await;
                        if msg.is_text() || msg.is_binary() {
                            sender.send(msg.clone()).await?;
                            // TODO:
                            let resp = connection.handle_message(msg).await;
                            match resp {
                                Ok((resp)) => {sender.send(resp);},
                                Err(err) => {sender.send(Message::Text(err.to_string()));},
                            };


                        } else if msg.is_close() {
                            break;
                        }
                    }
                    None => break,
                }
            }

            // _ = interval.tick() => {
            //     sender.lock().await.send(Message::Text("tick".to_owned())).await?;
            // }
        }
    }

    sender.lock().await.close().await?;

    Ok(())
}
