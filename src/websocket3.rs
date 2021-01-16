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

use crate::rpc_api::state::*;
use std::sync::Arc;
use std::{net::SocketAddr, time::Duration};
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tokio::sync::broadcast::Sender;
use tokio::sync::RwLock;
use tokio_tungstenite::{accept_async, tungstenite, WebSocketStream};
use tungstenite::{Error, Message};

/// A wrapper for WebSocketStream
#[derive(Debug)]
pub struct WsServer {
    listener: TcpListener,
}

/// WsConnection maintains state. When WsServer accept a new connection, a WsConnection will be returned.
#[derive(Debug)]
pub struct WsConnection {
    pub addr: SocketAddr,
    pub session: Arc<RwLock<SubscriptionSession>>,
}

impl WsServer {
    pub async fn bind<A: ToSocketAddrs>(addr: A) -> std::io::Result<Self> {
        let listener = TcpListener::bind(&addr).await?;

        Ok(Self { listener })
    }

    /// returns a WebSocketStream and corresponding connection as a state
    pub async fn accept(&self) -> std::io::Result<(WebSocketStream<TcpStream>, WsConnection)> {
        let (stream, addr) = self.listener.accept().await?;
        let stream = accept_async(stream).await.expect("Failed to accept");

        Ok((
            stream,
            WsConnection {
                addr,
                session: Arc::new(RwLock::new(Default::default())),
            },
        ))
    }
}

impl WsConnection {
    pub async fn handle_message(&self, msg: Message) -> Result<Message> {
        match msg {
            Message::Text(text) => {
                let msg = serde_json::from_str(&*text).map_err(ServiceError::JsonError);
                match msg {
                    Ok(msg) => {
                        // handle jsonrpc error
                        let res = handle_request(self.session.clone(), msg).await?;
                        Ok(Message::Text(serde_json::to_string(&res)?))
                    }
                    // handle json api error
                    Err(err) => Err(err),
                }
            }
            _ => unimplemented!(),
        }
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

async fn handle_kafka_message(
    session: Arc<RwLock<SubscriptionSession>>,
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

            let session = session.read().await;
            for (key, storage) in session.0.iter() {
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
