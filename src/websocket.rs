use crate::error::{Result, ServiceError};
use crate::kafka_api::KafkaStoragePayload;
use crate::message::{
    MethodCall, RequestMessage, ResponseMessage, SubscribedData, SubscribedMessage,
    SubscribedParams, Version,
};
use crate::rpc_api::SubscribedResult;
use crate::session::{ArcSessions, Session, StorageKeys};
use crate::util;
use futures::sink::SinkExt;
use log::*;
use rdkafka::message::OwnedMessage;
use rdkafka::Message as KafkaMessage;

use crate::rpc_api::state::*;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tokio::sync::broadcast::Sender;
use tokio::sync::RwLock;
use tokio_tungstenite::{accept_async, tungstenite, WebSocketStream};
use tungstenite::Message;

/// A wrapper for WebSocketStream
#[derive(Debug)]
pub struct WsServer {
    listener: TcpListener,
}

/// WsConnection maintains state. When WsServer accept a new connection, a WsConnection will be returned.
#[derive(Debug, Clone)]
pub struct WsConnection {
    pub addr: SocketAddr,
    pub sessions: ArcSessions,
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
                sessions: Arc::new(RwLock::new(Default::default())),
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
                        let res = handle_request(self.sessions.clone(), msg).await?;

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

async fn handle_request(sessions: ArcSessions, msg: RequestMessage) -> Result<ResponseMessage> {
    let session: Session = Session::from(&msg);
    // TODO: check jsonrpc error rather than json error
    let request: MethodCall =
        serde_json::from_str(&*msg.request).map_err(ServiceError::JsonError)?;

    // TODO: use hashmap rather than if-else
    if request.method == *"state_subscribeStorage" {
        util::handle_state_subscribeStorage(sessions, session, request).await
    } else {
        Err(ServiceError::JsonrpcError(
            jsonrpc_core::Error::method_not_found(),
        ))
    }
}

// transfer a kafka message to a group of SubscribedMessage according to session
pub async fn handle_kafka_message(
    session: ArcSessions,
    msg: OwnedMessage,
) -> Result<Vec<SubscribedMessage>> {
    // TODO: handle different topic and key for message
    // ignore msg which does not have a payload
    let payload = match msg.payload() {
        None => {
            warn!("Receive a kafka message whose payload is none: {:?}", msg);
            return Ok(vec![]);
        }
        Some(payload) => payload,
    };
    let payload: KafkaStoragePayload =
        serde_json::from_slice(payload).map_err(ServiceError::JsonError)?;

    let result: SubscribedResult = StateStorageResult::from(payload).into();

    let session = session.read().await;

    let mut msgs = vec![];
    for (key, (id, storage)) in session.iter() {
        let msg = match storage {
            // send the subscription data to this subscription unconditionally
            StorageKeys::All => {
                let data = SubscribedData {
                    jsonrpc: Some(Version::V2),
                    params: SubscribedParams {
                        // TODO: make sure the subscription could be client_id.
                        subscription: id.clone(),
                        result: result.clone(),
                    },
                };

                let data = serde_json::to_string(&data)?;

                SubscribedMessage {
                    id: key.client_id.clone(),
                    chain: key.chain_name.clone(),
                    data,
                }
            }
            // TODO: do filter for keys
            _ => unimplemented!(),
        };

        msgs.push(msg);
    }

    Ok(msgs)
}
