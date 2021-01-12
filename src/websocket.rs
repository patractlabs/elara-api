use actix::{Actor, StreamHandler};
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;

use serde_json::Value;
use std::collections::{HashMap, HashSet};

use crate::kafka::*;
use crate::message::*;
use std::borrow::Borrow;
use std::sync::Arc;

use tokio::sync::RwLock;

// TODO: we need to match the same error with substrate
#[derive(Debug)]
pub struct SubscriptionError {}

// TODO: choose better struct
// TODO: refine
// map (chain_name ++ method ++ client_id) -> (keys set)
#[derive(Debug, Default)]
pub struct SubscriptionRoute(pub RwLock<HashMap<RouteKey, StorageKeys<HashSet<String>>>>);

#[derive(Clone, Debug)]
pub enum StorageKeys<T> {
    All,
    Some(T),
}

/// RouteKey as a subscription session
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RouteKey {
    pub chain_name: String,
    pub method_name: String,
    pub client_id: String,
}

impl From<&RequestMessage> for RouteKey {
    fn from(msg: &RequestMessage) -> Self {
        return Self {
            chain_name: msg.chain.clone(),
            client_id: msg.id.clone(),
            method_name: msg.request.method.clone(),
        };
    }
}

/// One websocket connection per SubscriptionServer
pub struct SubscriptionServer {
    // 一个持久ws连接：
    // 一次订阅只能订阅一个链上的一种类型数据
    // 一个ws长连接可以订阅多个链多种类型数据，而一种类型数据可以订阅多次，订阅多次可能会有重复情况，
    // 因为根据subscriber ID来区分，确实会重复推送
    // 而取消订阅也可以多次，根据 subscriber ID 来区分
    //
    // 考虑需要处理的情况：
    // 首先是需要基本跟substrate返回值要一样
    // 处理重复订阅问题
    //
    // subscriptions:  map (chain_name ++ method ++ client_id) -> (keys set + request id)

    // kafka_consumer:
    // storage key -> vec<id> ++ subscription kafka data.
    // the vec represent subscription index
    // subscribed_keys: HashMap<String, Vec<String>>,
    // client_ids in the set subscribing all keys
    // subscribe_all_keys: HashSet<String>,
    // TODO: support multiple consumers
    consumer: Arc<KvConsumer>,
    route: SubscriptionRoute,
}

impl SubscriptionServer {
    /// create a subscription ws server and start kafka consumer client
    pub fn new(consumer: Arc<KvConsumer>) -> Self {
        let server = Self {
            consumer,
            route: Default::default(),
            // subscribed_keys: Default::default(),
            // subscribe_all_keys: Default::default(),
        };

        server
    }

    async fn handle_subscription(&mut self, text: String) -> Result<(), SubscriptionError> {
        let msg: RequestMessage =
            serde_json::from_str(&*text).map_err(|err| SubscriptionError {})?;

        let key: RouteKey = RouteKey::from(&msg);

        if msg.request.method == "state_subscribeStorage".to_string() {
            let storage_keys = match msg.request.params {
                // TODO:
                Params::None => {
                    return Err(SubscriptionError {});
                }
                // subscribe every block
                Params::Array(arr) if arr.is_empty() => StorageKeys::All,

                Params::Array(arr) => {
                    let len = arr.len();
                    let keys = arr
                        .iter()
                        .filter_map(|val| val.as_str().map(|val| val.to_string()))
                        .collect::<HashSet<String>>();

                    // TODO: keep same behavior with substrate
                    if len != keys.len() {
                        return Err(SubscriptionError {});
                    }
                    StorageKeys::Some(keys)
                }
                _ => {
                    return Err(SubscriptionError {});
                }
            };
            self.route.0.write().await.insert(key, storage_keys);
        } else {
            unimplemented!()
        }

        Ok(())
    }
}

impl Actor for SubscriptionServer {
    type Context = ws::WebsocketContext<Self>;
}

/// Handler for ws::Message message
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for SubscriptionServer {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => {
                // TODO: handle async and error

                self.handle_subscription(text.to_string());
            }
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            _ => (),
        }
    }
}

async fn index(
    req: HttpRequest,
    stream: web::Payload,
    consumer: web::Data<Arc<KvConsumer>>,
    // server: web::Data<Arc<SubscriptionServer>>,
) -> Result<HttpResponse, Error> {
    let server = SubscriptionServer::new(consumer.get_ref().clone());
    ws::start(server, &req, stream)
}

async fn welcome() -> &'static str {
    "Welcome!"
}

// TODO: config
pub async fn create_ws_server() -> std::io::Result<()> {
    HttpServer::new(|| {
        // TODO: config
        let consumer = Arc::new(KvConsumer::new(HashMap::new().into_iter(), LogLevel::Debug));
        App::new()
            .route("/ws", web::get().to(index))
            .route("/", web::get().to(welcome))
            .default_service(web::to(|| HttpResponse::NotFound()))
            .data(consumer.clone())
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
