use crate::error::{Result, ServiceError};
use crate::kafka_api::KafkaStoragePayload;
use crate::message::{
    MethodCall, RequestMessage, ResponseMessage, SubscribedData, SubscribedParams, Version,
};
use crate::rpc_api::SubscribedResult;
use crate::session::{Session, StorageKeys, SubscriptionSession};
use crate::util;
use futures::{sink::SinkExt, StreamExt};
use log::*;
use rdkafka::message::OwnedMessage;
use rdkafka::Message as KafkaMessage;
use std::io::Split;
use std::sync::Arc;
use std::{net::SocketAddr, time::Duration};
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tokio::sync::RwLock;
use tokio_tungstenite::{accept_async, tungstenite, WebSocketStream};
use tungstenite::{Error, Message};

#[derive(Debug)]
pub struct WsServer {
    listener: TcpListener,
}

#[derive(Debug)]
pub struct WsConnection {
    pub stream: WebSocketStream<TcpStream>,
    pub addr: SocketAddr,
    pub session: SubscriptionSession,
}

impl WsServer {
    pub async fn bind<A: ToSocketAddrs>(addr: A) -> std::io::Result<Self> {
        let listener = TcpListener::bind(&addr).await?;

        Ok(Self { listener })
    }

    pub async fn accept(&self) -> std::io::Result<WsConnection> {
        let (stream, addr) = self.listener.accept().await?;
        let stream = accept_async(stream).await.expect("Failed to accept");

        Ok(WsConnection {
            stream,
            addr,
            session: Default::default(),
        })
    }
}

async fn handle_request(
    session: Arc<RwLock<SubscriptionSession>>,
    msg: RequestMessage,
) -> Result<ResponseMessage> {
    let key: Session = Session::from(&msg);
    // TODO: check jsonrpc error rather than json error
    let request: MethodCall =
        serde_json::from_str(&*msg.request).map_err(ServiceError::JsonError)?;

    // TODO: use hashmap rather than if-else
    if request.method == *"state_subscribeStorage" {
        util::handle_state_subscribeStorage(session, key, request).await
    } else {
        return Err(ServiceError::JsonrpcError(
            jsonrpc_core::Error::method_not_found(),
        ));
    }
}

async fn handle_message(
    session: Arc<RwLock<SubscriptionSession>>,
    msg: Message,
) -> Result<Message> {
    match msg {
        Message::Text(text) => {
            let msg = serde_json::from_str(&*text).map_err(ServiceError::JsonError);
            match msg {
                Ok(msg) => {
                    // handle jsonrpc error
                    let res = handle_request(session, msg).await?;
                    Ok(Message::Text(serde_json::to_string(&res)?))
                }
                // handle json api error
                Err(err) => Err(err),
            }
        }
        _ => unimplemented!(),
    }
}

use crate::rpc_api::state::*;
use tokio::sync::broadcast::Sender;

async fn handle_kafka_message(
    route: Arc<RwLock<SubscriptionSession>>,
    msg: Option<OwnedMessage>,
    subscription_message: &mut Sender<SubscribedData>,
) -> Result<()> {
    match msg {
        // TODO:
        // kafka consumer closed
        None => Ok(()),
        Some(msg) => {
            info!("{:?}", msg);
            // TODO: handle different topic and key for message
            if msg.payload().is_none() {
                return Ok(());
            }
            let payload = msg.payload().unwrap();
            let payload: KafkaStoragePayload =
                serde_json::from_slice(payload).map_err(ServiceError::JsonError)?;

            let result: SubscribedResult = StateStorageResult::from(payload).into();

            let route = route.read().await;
            for (key, storage) in route.0.iter() {
                match storage {
                    // send the subscription data to this subscription unconditionally
                    StorageKeys::All => {
                        subscription_message.send(SubscribedData {
                            jsonrpc: Some(Version::V2),
                            params: SubscribedParams {
                                // TODO:
                                subscription: key.client_id.clone(),
                                result: result.clone(),
                            },
                        });
                    }
                    // TODO: do filter
                    _ => {}
                };
            }

            Ok(())
        }
    }
}

impl WsConnection {
    async fn accept_connection(self) {
        if let Err(e) = self.handle_connection().await {
            match e {
                Error::ConnectionClosed | Error::Protocol(_) | Error::Utf8 => (),
                err => error!("Error processing connection: {}", err),
            }
        }
    }

    async fn handle_connection(self) -> tungstenite::Result<()> {
        info!("New WebSocket connection: {}", self.addr);
        let (mut ws_sender, mut ws_receiver) = self.stream.split();
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
}
