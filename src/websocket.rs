use actix::{Actor, StreamHandler};
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;

use std::collections::{HashMap, HashSet};

use crate::message::*;

type Subscription = String;
type RouteId = String;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Subscribed {
    client_ids: Vec<String>,
}

/// Define HTTP actor
#[derive(Debug, Default)]
pub struct WsSubscribeServer {
    /// 多个 RouteId 订阅公用一个Subscription
    // subscriptions: HashMap<Subscription, HashSet<RouteId>>,

    // TODO: support multiple consumers
    // kafka_consumer:
    // storage key -> vec<id> ++ subscription kafka data
    subscribed_keys: HashMap<String, Vec<String>>,
    // client_ids in the set subscribing all keys
    subscribe_all_keys: HashSet<String>,
}

impl WsSubscribeServer {
    fn handle_subscribe(&mut self, text: String) {
        let msg: RequestMessage = serde_json::from_str(&*text)?;

        // TODO: add route id
        if msg.request.method == "state_subscribeStorage".to_string() {
        } else {
            unimplemented!()
        }
    }
}

impl Actor for WsSubscribeServer {
    type Context = ws::WebsocketContext<Self>;
}

/// Handler for ws::Message message
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsSubscribeServer {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => {
                text.to_string();
            }
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            _ => (),
        }
    }
}

async fn index(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    let resp = ws::start(WsSubscribeServer::default(), &req, stream);
    println!("{:?}", resp);
    resp
}

pub async fn main() -> std::io::Result<()> {
    let res = HttpServer::new(|| App::new().route("/ws/", web::get().to(index)))
        .bind("127.0.0.1:8080")?
        .run()
        .await;

    println!("{:?}", res);

    res
}
