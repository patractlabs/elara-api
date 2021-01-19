use log::*;

use elara_api::kafka::{KVSubscriber, KvConsumer, LogLevel, OwnedMessage};
use elara_api::websocket::{handle_kafka_message, WsConnection, WsServer};
use futures::{Sink, SinkExt, Stream, StreamExt};
use std::sync::Arc;
use tokio::net::{ TcpStream};
use tokio::sync::broadcast::{Receiver, Sender};

use tokio_tungstenite::{tungstenite, WebSocketStream};
use tungstenite::{Error, Message};

use elara_api::config::*;
use elara_api::error::Result;
use tokio::sync::Mutex;
use futures::stream::{SplitSink, SplitStream};
use elara_api::message::ResponseErrorMessage;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    // TODO; refine config
    let path = "bin/config.toml";
    info!("Load config from: {}", path);
    let cfg = load_config(path).expect("Illegal config path");
    let cfg: Config = toml::from_str(&*cfg).expect("Config is illegal");

    debug!("Load config: {:#?}", &cfg);
    let consumer = KvConsumer::new(cfg.kafka.into_iter(), LogLevel::Debug);

    info!("Subscribing kafka topic `{}`", "polkadot");
    consumer.subscribe(&["polkadot"])?;

    let subscriber = KVSubscriber::new(Arc::new(consumer), 100);
    // start subscribing some topics
    subscriber.start();

    let addr = cfg.ws.addr.as_str();
    let server = WsServer::bind(addr)
        .await
        .expect(&*format!("Cannot listen {}", addr));
    info!("Started ws server at {}", addr);

    // accept a new connection
    while let Ok((stream, connection)) = server.accept().await {
        handle_connection(
            connection,
            stream,
            subscriber.subscribe(),
        ).await;
    }

    Ok(())
}

fn handle_ws_error(err: tungstenite::Result<()>) {
    if let Err(e) = err {
        match e {
            Error::ConnectionClosed | Error::Protocol(_) | Error::Utf8 => (),
            err => error!("Error processing connection: {}", err),
        }
    }
}

async fn handle_connection(
    connection: WsConnection,
    stream: WebSocketStream<TcpStream>,
    kafka_receiver: Receiver<OwnedMessage>,
)  {
    info!("New WebSocket connection: {}", connection.addr);
    let (sender, receiver) = stream.split();
    let sender = Arc::new(Mutex::new(sender));

    tokio::spawn(handle_request(connection.clone(), sender.clone(), receiver));
    tokio::spawn(start_pushing_service(connection, sender, kafka_receiver));
}


// push subscription data for the connection
async fn start_pushing_service(
    connection: WsConnection,
    ws_sender: Arc<Mutex<SplitSink<WebSocketStream<TcpStream>, Message>>>,
    mut kafka_receiver: Receiver<OwnedMessage>,
) {
    // receive kafka message in background and send them if possible
    tokio::spawn(async move {
        while let Ok(msg) = kafka_receiver.recv().await {
            debug!("Receive a kafka message: {:?}", msg);
            let res = handle_kafka_message(connection.sessions.clone(), msg).await;
            match res {
                Err(ref err) => {
                    warn!(
                        "Error occurred when group data according to subscription params: {:?}",
                        err
                    );
                }

                Ok(msgs) => {
                    let mut sender = ws_sender.lock().await;
                    for msg in msgs.iter() {
                        let msg = serde_json::to_string(msg).expect("Won't be error");
                        let res = sender.feed(Message::Text(msg)).await;
                        // TODO: refine it. We need to exit if connection is closed
                        handle_ws_error(res);
                    }
                    let res = sender.flush().await;
                    handle_ws_error(res);
                }
            }
        }
    });
}

async fn handle_request(
    connection: WsConnection,
    ws_sender: Arc<Mutex<SplitSink<WebSocketStream<TcpStream>, Message>>>,
    mut ws_receiver: SplitStream<WebSocketStream<TcpStream>>,
) {
    loop {
        let msg = ws_receiver.next().await;
        match msg {
            Some(Ok(msg)) => {
                let mut sender = ws_sender.lock().await;
                if msg.is_close() {
                    break;

                } else if msg.is_empty() {
                    // do nothing

                } else if msg.is_text() || msg.is_binary() {
                    let resp = connection.handle_message(msg).await;
                    // do response
                    let res = match resp {
                        Ok(resp) => sender.send(resp).await,
                        Err(err) => {
                            let err  = serde_json::to_string(&ResponseErrorMessage::from(err)).unwrap();
                            sender.send(Message::Text(err)).await
                        }
                    };

                    // handle response error
                    match res {
                        Ok(()) => {}
                        Err(err) => {
                            warn!("Error occurred when response: {}", err);
                        }
                    };
                }
            }

            // closed connection
            Some(Err(Error::ConnectionClosed)) | None => {
                break
            },

            Some(Err(err)) => {
                warn!("{}", err);
            }
        }
    }

    match ws_sender.lock().await.close().await {
        Ok(()) => {},
        Err(err) => {
            warn!(
                "Error occurred when closed connection to {}: {}",
                connection.addr, err
            );
        }
    };
}
