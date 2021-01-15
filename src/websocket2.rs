use crate::kafka::KvConsumer;
use crate::message::RequestMessage;

use futures::{sink::SinkExt, StreamExt};
use jsonrpc_core::MethodCall;
use log::*;
use std::collections::HashSet;
use std::sync::Arc;
use std::{net::SocketAddr, time::Duration};
use tokio::macros::support::Future;
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};

use tokio_tungstenite::{accept_async, tungstenite};
use tungstenite::{Error, Message};

use crate::error::ServiceError;
use crate::message::*;
use crate::route::{RouteKey, StorageKeys, SubscriptionRoute};
use rdkafka::message::OwnedMessage;
use rdkafka::Message as KafkaMessage;
use std::borrow::{Borrow, BorrowMut};
use std::collections::hash_map::RandomState;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::{Mutex, RwLock};

#[derive(Clone)]
pub struct WsServer {
    route: Arc<RwLock<SubscriptionRoute>>,
    // receive kafka stream data
    msg_receiver: Arc<Mutex<Receiver<OwnedMessage>>>,

    subscription_sender: Arc<Mutex<Sender<SubscribedData>>>,
}

impl WsServer {
    // TODO: config
    pub fn new(
        msg_receiver: Receiver<OwnedMessage>,
        subscription_sender: Sender<SubscribedData>,
    ) -> Self {
        Self {
            // consumer,
            route: Default::default(),
            msg_receiver: Arc::new(Mutex::new(msg_receiver)),
            subscription_sender: Arc::new(Mutex::new(subscription_sender)),
        }
    }

    async fn handle_kafka_message(
        &mut self,
        msg: Option<OwnedMessage>,
        subscription_message: &Sender<SubscribedData>,
    ) {
        match msg {
            // kafka consumer closed
            None => return,
            Some(msg) => {
                info!("{:?}", msg);
                // TODO: handle different topic and key for message
                if msg.payload().is_none() {
                    return;
                }
                let payload = msg.payload().unwrap();
                let payload: Result<KafkaStoragePayload, _> = serde_json::from_slice(payload);

                let payload = match payload {
                    Err(err) => {
                        warn!("{}", err.to_string());
                        return;
                    }

                    Ok(payload) => payload,
                };

                let route = self.route.read().await;
                for (key, storage) in route.0.iter() {
                    match storage {
                        // send the subscription data to this subscription unconditionally
                        StorageKeys::All => {
                            subscription_message.send(SubscribedData {
                                jsonrpc: Some(Version::V2),
                                params: SubscribedParams {
                                    // TODO:
                                    subscription: key.client_id.clone(),
                                    result: vec![],
                                },
                            });
                        }
                        // TODO: do filter
                        _ => {}
                    };
                }
            }
        };
    }

    // subscribe_kafka will blocking until all kafka data have been consumed.
    pub async fn subscribe_kafka(
        &mut self,
        mut kafka_message: Receiver<OwnedMessage>,
        mut subscription_message: Sender<SubscribedData>,
    ) {
        loop {
            // TODO: handle different topic message
            tokio::select! {
                msg = kafka_message.recv() => {
                   self.handle_kafka_message(msg, &subscription_message).await;
                },

                // msg = subscription_message.send() => {
                //     match msg {
                //         None => {}
                //     }
                // }
            }
        }

        // loop {
        //     let msg = self.msg_receiver.lock().await.recv().await;
        //     match msg {
        //         None => {return },
        //         Some(msg) => {
        //             // TODO: send data according to route
        //             let route =  self.route.read().await.0.borrow();
        //
        //         }
        //     }
        // }
    }

    pub async fn bind<A: ToSocketAddrs>(&self, addr: A) {
        let listener = TcpListener::bind(&addr).await.expect("Can't listen");
        while let Ok((stream, _)) = listener.accept().await {
            let peer = stream
                .peer_addr()
                .expect("connected streams should have a peer address");
            info!("Peer address: {}", peer);

            let mut server = self.clone();
            tokio::spawn(async move { server.accept_connection(peer, stream).await });
        }
    }

    async fn accept_connection(&mut self, peer: SocketAddr, stream: TcpStream) {
        if let Err(e) = Self::handle_connection(peer, stream).await {
            match e {
                Error::ConnectionClosed | Error::Protocol(_) | Error::Utf8 => (),
                err => error!("Error processing connection: {}", err),
            }
        }
    }

    #[allow(non_snake_case)]
    async fn handle_state_subscribeStorage(
        &mut self,
        key: RouteKey,
        request: MethodCall,
    ) -> Result<ResponseMessage, ServiceError> {
        let params: Vec<Vec<String>> = request.params.parse()?;
        let storage_keys = match params {
            arr if arr.is_empty() || arr.len() > 1 => {
                return Err(ServiceError::JsonrpcError(
                    jsonrpc_core::Error::invalid_params("some params are invalid"),
                ));
            }
            arr if arr[0].is_empty() => StorageKeys::All,
            arrs => {
                let arr = &arrs[0];
                let len = arr.len();
                let keys = arr
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<HashSet<String>>();

                // TODO: keep same behavior with substrate
                if len != keys.len() {
                    return Err(ServiceError::JsonrpcError(
                        jsonrpc_core::Error::invalid_params("some params are invalid"),
                    ));
                }
                StorageKeys::Some(keys)
            }
        };

        {
            let mut route = self.route.write().await;
            route.0.insert(key.clone(), storage_keys);
        }

        let result = serde_json::to_string(&Output::from(
            // state_subscribeStorage's result is subscription id
            // TODO: make sure the subscription id
            Ok(Value::String(key.client_id.clone())),
            request.id,
            Some(Version::V2),
        ))?;

        Ok(ResponseMessage {
            id: key.client_id,
            chain: key.chain_name,
            result,
        })
    }

    async fn handle_request(
        &mut self,
        msg: RequestMessage,
    ) -> Result<ResponseMessage, ServiceError> {
        let key: RouteKey = RouteKey::from(&msg);
        // TODO: check jsonrpc error rather than json error
        let request: MethodCall =
            serde_json::from_str(&*msg.request).map_err(ServiceError::JsonError)?;

        // TODO: use hashmap rather than if-else
        if request.method == *"state_subscribeStorage" {
            self.handle_state_subscribeStorage(key, request).await
        } else {
            return Err(ServiceError::JsonrpcError(
                jsonrpc_core::Error::method_not_found(),
            ));
        }
    }

    async fn handle_message(&mut self, msg: Message) -> Result<Message, ServiceError> {
        match msg {
            Message::Text(text) => {
                let msg = serde_json::from_str(&*text).map_err(ServiceError::JsonError);
                match msg {
                    Ok(msg) => {
                        // handle jsonrpc error
                        let res = self.handle_request(msg).await?;
                        Ok(Message::Text(serde_json::to_string(&res)?))
                    }
                    // handle json api error
                    Err(err) => Err(err),
                }
            }
            _ => unimplemented!(),
        }
    }

    async fn handle_connection(peer: SocketAddr, stream: TcpStream) -> tungstenite::Result<()> {
        let ws_stream = accept_async(stream).await.expect("Failed to accept");
        info!("New WebSocket connection: {}", peer);
        let (mut sender, mut receiver) = ws_stream.split();
        let mut interval = tokio::time::interval(Duration::from_millis(1000));

        // Echo incoming WebSocket messages and send a message periodically every second.
        loop {
            tokio::select! {
                msg = receiver.next() => {
                    match msg {
                        Some(msg) => {
                            let msg = msg?;
                            if msg.is_text() || msg.is_binary() {
                                sender.send(msg).await?;

                            } else if msg.is_close() {
                                break;
                            }
                        }
                        None => break,
                    }
                }
                _ = interval.tick() => {
                    sender.send(Message::Text("tick".to_owned())).await?;
                }
            }
        }

        sender.close().await?;
        Ok(())
    }
}
