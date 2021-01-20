use crate::error::{Result, ServiceError};
use crate::kafka_api::KafkaStoragePayload;
use crate::message::{
    Failure, Id, MethodCall, RequestMessage, ResponseMessage, SubscribedData, SubscribedMessage,
    SubscribedParams, Success, Version,
};
use crate::rpc_api::SubscribedResult;
use crate::session::{Session, StorageKeys, StorageSessions};
use crate::util;
use rdkafka::message::OwnedMessage;
use rdkafka::Message as KafkaMessage;

use crate::rpc_api::state::*;
use futures::stream::{SplitSink, SplitStream};
use futures::StreamExt;
use std::net::SocketAddr;
use std::ops::DerefMut;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tokio::sync::{Mutex, RwLock};
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
    pub sender: Arc<Mutex<SplitSink<WebSocketStream<TcpStream>, Message>>>,
    pub receiver: Arc<Mutex<SplitStream<WebSocketStream<TcpStream>>>>,

    // TODO: add other session type
    pub storage_sessions: Arc<RwLock<StorageSessions>>,
}

impl WsServer {
    pub async fn bind<A: ToSocketAddrs>(addr: A) -> std::io::Result<Self> {
        let listener = TcpListener::bind(&addr).await?;

        Ok(Self { listener })
    }

    /// returns a WebSocketStream and corresponding connection as a state
    pub async fn accept(&self) -> tungstenite::Result<WsConnection> {
        let (stream, addr) = self.listener.accept().await?;
        let stream = accept_async(stream).await?;
        let (sender, receiver) = stream.split();
        Ok(WsConnection {
            addr,
            sender: Arc::new(Mutex::new(sender)),
            receiver: Arc::new(Mutex::new(receiver)),
            storage_sessions: Arc::new(RwLock::new(StorageSessions::new())),
        })
    }
}

impl WsConnection {
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub async fn handle_message(&self, msg: impl Into<String>) -> Result<String> {
        let msg = msg.into();
        let msg = serde_json::from_str(&*msg).map_err(ServiceError::JsonError);
        match msg {
            Ok(msg) => {
                // handle jsonrpc error
                let session: Session = Session::from(&msg);
                let res = self.handle_request(session.clone(), msg).await;
                let result = match res {
                    Ok(success) => serde_json::to_string(&success)
                        .expect("serialize result for success response"),
                    Err(failure) => serde_json::to_string(&failure)
                        .expect("serialize result for failure response"),
                };

                let response = ResponseMessage {
                    id: session.client_id,
                    chain: session.chain_name,
                    result,
                };

                Ok(serde_json::to_string(&response).expect("serialize response for elara"))
            }
            Err(err) => Err(err),
        }
    }

    async fn handle_request(
        &self,
        session: Session,
        msg: RequestMessage,
    ) -> std::result::Result<Success, Failure> {
        let request = serde_json::from_str::<MethodCall>(&*msg.request).map_err(|_| Failure {
            jsonrpc: None,
            error: jsonrpc_core::Error::parse_error(),
            id: Id::Null,
        })?;

        let id = request.id.clone();
        let storage_sessions = self.storage_sessions.clone();
        let mut storage_sessions = storage_sessions.write().await;

        // TODO: use hashmap rather than if-else
        let res = if request.method == *"state_subscribeStorage" {
            util::handle_state_subscribeStorage(storage_sessions.deref_mut(), session, request)
        } else if request.method == *"state_unsubscribeStorage" {
            util::handle_state_unsubscribeStorage(storage_sessions.deref_mut(), session, request)
        } else {
            Err(jsonrpc_core::Error::method_not_found())
        };

        res.map_err(|err| Failure {
            jsonrpc: None,
            error: err,
            id,
        })
    }
}

// transfer a kafka message to a group of SubscribedMessage according to storage session
pub fn collect_subscribed_storage(
    sessions: &StorageSessions,
    msg: OwnedMessage,
) -> Result<Vec<SubscribedMessage>> {
    // ignore msg which does not have a payload
    let payload = match msg.payload() {
        Some(payload) => payload,
        _ => unreachable!(),
    };
    let payload: KafkaStoragePayload =
        serde_json::from_slice(payload).map_err(ServiceError::JsonError)?;

    // result for subscribing all storages
    let result: SubscribedResult = StateStorageResult::from(&payload).into();

    let mut msgs = vec![];
    for (subscription_id, (session, storage)) in sessions.iter() {
        let data: String = match storage {
            // send the subscription data to this subscription unconditionally
            StorageKeys::All => {
                let data = SubscribedData {
                    jsonrpc: Some(Version::V2),
                    // TODO:
                    method: "".to_string(),
                    params: SubscribedParams {
                        // TODO: make sure the subscription could be client_id.
                        result: result.clone(),
                        subscription: subscription_id.clone(),
                    },
                };

                serde_json::to_string(&data).expect("serialize a subscribed data")
            }

            // TODO: do filter for keys
            StorageKeys::Some(_keys) => {
                unimplemented!();
            }
        };

        msgs.push(SubscribedMessage {
            id: session.client_id.clone(),
            chain: session.chain_name.clone(),
            data,
        });
    }

    Ok(msgs)
}
