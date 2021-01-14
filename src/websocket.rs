use actix::{Actor, ActorContext, StreamHandler, AsyncContext, Context};
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;

use std::collections::{HashMap, HashSet};

use crate::kafka::*;
use crate::message::*;

use std::sync::Arc;

use crate::error::ServiceError;

// TODO: choose better struct
// TODO: refine
// map (chain_name ++ client_id) -> (keys set)
#[derive(Debug, Default)]
pub struct SubscriptionRoute(pub HashMap<RouteKey, StorageKeys<HashSet<String>>>);

#[derive(Clone, Debug)]
pub enum StorageKeys<T> {
    All,
    Some(T),
}

/// RouteKey as a subscription session
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RouteKey {
    pub chain_name: String,
    pub client_id: String,
}

impl From<&RequestMessage> for RouteKey {
    fn from(msg: &RequestMessage) -> Self {
        Self {
            chain_name: msg.chain.clone(),
            client_id: msg.id.clone(),
        }
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

    #[allow(non_snake_case)]
    fn handle_state_subscribeStorage(
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

        self.route.0.insert(key.clone(), storage_keys);
        // TODO: response the request

        let result =  serde_json::to_string(&Output::from(
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

    fn handle_request(
        &mut self,
        msg: RequestMessage,
        ctx: &mut <Self as Actor>::Context,
    ) -> Result<(), ServiceError> {

        let key: RouteKey = RouteKey::from(&msg);
        // TODO: check jsonrpc error rather than json error
        let request: MethodCall =
            serde_json::from_str(&*msg.request).map_err(ServiceError::JsonError)?;

        // TODO: use hashmap rather than if-else
        let res = if request.method == *"state_subscribeStorage" {
            self.handle_state_subscribeStorage(key, request)?
        } else {
            return Err(ServiceError::JsonrpcError(
                jsonrpc_core::Error::method_not_found(),
            ));
        };

        ctx.text(serde_json::to_string(&res)?);
        Ok(())
    }
}

impl Actor for SubscriptionServer {
    type Context = ws::WebsocketContext<Self>;
}

pub struct KafkaService;

impl Actor for KafkaService {
    type Context = Context<Self>;
}

impl StreamHandler<KafkaResult<OwnedMessage>> for KafkaService {
    /// Method is called for every message received by this Actor
    fn handle(&mut self, item: KafkaResult<OwnedMessage>, ctx: &mut Self::Context) {}

    /// Method is called when stream get polled first time.
    fn started(&mut self, ctx: &mut Self::Context) {
        dbg!("started KafkaResult<OwnedMessage>");
        ctx.add_stream(
            self.consumer
        );
    }

    /// Method is called when stream finishes.
    ///
    /// By default this method stops actor execution.
    fn finished(&mut self, ctx: &mut Self::Context) {}
}


use once_cell::sync::Lazy;
use std::sync::Mutex;
use futures::Stream;
use rdkafka::message::OwnedMessage;

static GLOBAL_DATA: Lazy<Mutex<i32>> = Lazy::new(|| Mutex::new(0));

#[derive(Debug, Clone)]
struct Ping(String, i32);


impl StreamHandler<KafkaResult<OwnedMessage>> for SubscriptionServer {
    /// Method is called for every message received by this Actor
    fn handle(&mut self, item: KafkaResult<OwnedMessage>, ctx: &mut Self::Context) {
        match item {
            Err(err) => {
                // TODO:
            }

            Ok(msg) => {
                println!("{:?}", msg);
            }
        }
    }

    /// Method is called when stream get polled first time.
    fn started(&mut self, ctx: &mut Self::Context) {
        dbg!("started KafkaResult<OwnedMessage>");

        tokio::spawn(async {

        });

        ctx.add_stream(
            self.consumer.stream()
        );
    }


    /// Method is called when stream finishes.
    ///
    /// By default this method stops actor execution.
    fn finished(&mut self, ctx: &mut Self::Context) {}
}

/// Handler for ws::Message message
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for SubscriptionServer {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => {
                let msg = serde_json::from_str(&*text).map_err(ServiceError::JsonError);
                match msg {
                    Ok(msg) => {
                        // handle jsonrpc error
                        let _res = self.handle_request(msg, ctx);
                    }
                    // handle json api error
                    Err(err) => ctx.text(err.to_string()),
                }
            }
            // TODO: support compression
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.terminate();
            }
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
            .default_service(web::to(HttpResponse::NotFound))
            .data(consumer)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
