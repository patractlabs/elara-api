use crate::error::{Result, ServiceError};
use crate::kafka_api::KafkaStoragePayload;
use crate::message::{
    Failure, Id, MethodCall, RequestMessage, ResponseMessage, SubscribedData, SubscribedMessage,
    SubscribedParams, Success, Version,
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
    pub async fn accept(&self) -> tungstenite::Result<(WebSocketStream<TcpStream>, WsConnection)> {
        let (stream, addr) = self.listener.accept().await?;
        let stream = accept_async(stream).await?;
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

                        // TODO: handle serde_json error;
                        Ok(Message::Text(
                            serde_json::to_string(&response)
                                .expect("ResponseMessage won't be error"),
                        ))
                    }
                    // handle json api error
                    Err(err) => Err(err),
                }
            }

            Message::Binary(_bytes) => {
                unimplemented!()
            }
            // TODO:
            _ => unreachable!(),
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

// transfer a kafka message to a group of SubscribedMessage according to session
pub async fn handle_kafka_message(
    sessions: &StorageSessions,
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

    // result for subscribing all storages
    let result: SubscribedResult = StateStorageResult::from(payload).into();

    let mut msgs = vec![];
    for (subscription_id, (session, storage)) in sessions.iter() {
        let data: String = match storage {
            // send the subscription data to this subscription unconditionally
            StorageKeys::All => {
                let data = SubscribedData {
                    jsonrpc: Some(Version::V2),
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
                // let data = SubscribedData {
                //     jsonrpc: Some(Version::V2),
                //     params: SubscribedParams {
                //         subscription: subscription_id.clone(),
                //         result,
                //     }
                // }
                unimplemented!()
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
