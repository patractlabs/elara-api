use crate::error::{Result, ServiceError};
use crate::kafka_api::KafkaStoragePayload;
use crate::message::{
    MethodCall, RequestMessage, ResponseMessage, SubscribedData, SubscribedMessage,
    SubscribedParams, Version,
};
use crate::rpc_api::SubscribedResult;
use crate::session::{Session, StorageKeys, StorageSessions};
use crate::util;
use log::*;
use rdkafka::message::OwnedMessage;
use rdkafka::Message as KafkaMessage;

use crate::rpc_api::state::*;
use std::net::SocketAddr;
use std::ops::DerefMut;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
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
    // TODO: add other session type
    pub storage_sessions: Arc<RwLock<StorageSessions>>,
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
                storage_sessions: Arc::new(RwLock::new(StorageSessions::new())),
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
                        let res = self.handle_request(msg).await?;
                        // TODO: handle serde_json error;
                        Ok(Message::Text(
                            serde_json::to_string(&res).expect("ResponseMessage won't be error"),
                        ))
                    }
                    // handle json api error
                    Err(err) => Err(err),
                }
            }

            Message::Binary(bytes) => {
                unimplemented!()
            }
            // TODO:
            _ => unreachable!(),
        }
    }

    async fn handle_request(&self, msg: RequestMessage) -> Result<ResponseMessage> {
        let session: Session = Session::from(&msg);
        // TODO: check jsonrpc error rather than json error
        let request: MethodCall =
            serde_json::from_str(&*msg.request).map_err(ServiceError::JsonError)?;

        let storage_sessions = self.storage_sessions.clone();
        let mut storage_sessions = storage_sessions.write().await;

        // TODO: use hashmap rather than if-else
        if request.method == *"state_subscribeStorage" {
            util::handle_state_subscribeStorage(storage_sessions.deref_mut(), session, request)
        } else if request.method == *"state_unsubscribeStorage" {
            util::handle_state_unsubscribeStorage(storage_sessions.deref_mut(), session, request)
        } else {
            Err(ServiceError::JsonrpcError(
                jsonrpc_core::Error::method_not_found(),
            ))
        }
    }
}

// transfer a kafka message to a group of SubscribedMessage according to session
pub async fn handle_kafka_message(
    sessions: &Arc<RwLock<StorageSessions>>,
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

    let sessions = sessions.read().await;

    let mut msgs = vec![];
    for (key, (id, storage)) in sessions.iter() {
        let msg = match storage {
            // send the subscription data to this subscription unconditionally
            StorageKeys::All => {
                let data = SubscribedData {
                    jsonrpc: Some(Version::V2),
                    params: SubscribedParams {
                        // TODO: make sure the subscription could be client_id.
                        result: result.clone(),
                        subscription: key.clone(),
                    },
                };

                let data = serde_json::to_string(&data)?;

                SubscribedMessage {
                    id: id.client_id.clone(),
                    chain: id.chain_name.clone(),
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
