use std::collections::HashMap;
use super::validator::Validator;
use super::request::*;
use super::curl::*;
use chrono::{Utc};
use log::*;
use actix_web::{get, middleware, web, App, HttpRequest, HttpResponse, HttpServer};
use actix_web::web::Data;
use std::sync::{Arc};
use crate::http::server::{MessageSender, ReqMessage, parseIp};
use futures_util::StreamExt;
use serde_json::{Value};

#[derive(Clone)]
pub struct ActixWebServer {
    target: Arc<HashMap<String, String>>,
    validator: Validator,
    client: RequestCurl,
    txCh: MessageSender<(String, String)>
}

impl ActixWebServer {
    pub fn new(target: Arc<HashMap<String, String>>, vali: Validator, tx: MessageSender<(String, String)>) -> Self {
        ActixWebServer{target: target, validator: vali, client: RequestCurl, txCh: tx}
    }

    pub fn SendMsg(&self, key: &str, info: &str) -> bool {
        if self.txCh.Send((key.to_string(), info.to_string())) {
            return true;
        }
        false
    }
}

pub fn RunServer(svr: ActixWebServer, port: String) -> std::io::Result<()> {
    actix_web::rt::System::new("proxy").block_on(async move {
        HttpServer::new(move || {
            App::new()
                .data(svr.clone())
                .wrap(middleware::DefaultHeaders::new().header("X-Version", "0.2"))
                .wrap(middleware::Compress::default())
                .wrap(middleware::Logger::default())
                .service(index)
                .service(web::resource("/{chain}/{pid}").route(web::post().to(transfer)))
        })
        .bind(format!("0.0.0.0:{}", port))?
        .run()
        .await
    })
}

#[get("/actix")]
async fn index() -> &'static str {
    "Hello world!\r\n"
}

async fn transfer(obj: Data<ActixWebServer>, req: HttpRequest, web::Path((chain, pid)): web::Path<(String, String)>, mut payload: web::Payload) -> HttpResponse {
        // verify chain and project
        {
            let path = "/".to_string()+&chain+"/"+&pid;
            if !obj.validator.CheckLimit(path) {
                error!("project not exist");
                let body = serde_json::to_string(&ApiResp{code:-1, message:"chain or project not match".to_string(), data:None}).unwrap();
                return HttpResponse::Ok()
                        .content_type("application/json")
                        .body(body);
            }
        }
        if !obj.target.contains_key(&chain) {
            error!("chain {} not configured", chain);
            let body = serde_json::to_string(&ApiResp{code:-3, message:"chain not config".to_string(), data:None}).unwrap();
            return HttpResponse::Ok()
                    .content_type("application/json")
                    .body(body);
        }
        let chainAddr = obj.target[&chain].clone();

        let mut bytes = web::BytesMut::new();
        while let Some(item) = payload.next().await {
            bytes.extend_from_slice(&item.unwrap());
        }
        let contents = String::from_utf8_lossy(&bytes).to_string();
        debug!("content:{}", contents);

        let start = Utc::now().timestamp_millis();
        let (resp, ok) = obj.client.Rpc(&chainAddr, contents.clone());
        if ok {
            let end = Utc::now().timestamp_millis();
            let msg = parseActixRequest(&req, &resp.data.unwrap_or("null".to_string()), &chain, &pid, &contents, start, end);
            let info = serde_json::to_string(&msg).unwrap();
            // todo: async, do send in other thread
            obj.SendMsg("request", &info);
            return HttpResponse::Ok()
                    .content_type("application/json")
                    .body(resp.mssage);
        }
        let body = serde_json::to_string(&ApiResp{code:-4, message:"no response".to_string(), data:None}).unwrap();
        return HttpResponse::Ok()
                .content_type("application/json")
                .body(body);
}

fn parseActixRequest(req: &HttpRequest, resp: &str, chain: &str, pid: &str, param: &str, start: i64, end: i64) -> ReqMessage {
    let sliceHead = resp.split("\r\n").collect::<Vec<&str>>();
    let rHeads = sliceHead[0].split(" ").collect::<Vec<&str>>();
    let length = sliceHead.len()-3;
    let mut band = String::new();
    for i in 1..length {
        let ok = sliceHead[i].contains("Content-Length");
        debug!("scan {} {}", sliceHead[i], ok);
        if ok {
            let dataLen = sliceHead[i].split(" ").collect::<Vec<&str>>();
            debug!("{:?}", dataLen);
            band = dataLen[1].to_string();
            break;
        }
    }
    
    let mut msg = ReqMessage::new();
    msg.protocol = rHeads[0].to_string(); //rocket cannot get request protocol
    let mut clientIp = "127.127.127.127".to_string();
    if let Some(forwards) = req.headers().get("x-forward-for") {
        clientIp = parseIp(forwards.to_str().unwrap_or(""), clientIp.clone());
    }
    msg.ip = clientIp;
    msg.header = format!("{:?}", req.headers());
    msg.chain = chain.to_string();
    msg.pid = pid.to_string();
    let deserialized: Value = serde_json::from_str(&param).unwrap();
    let noMethod = Value::String("no method".to_string());
    let method = deserialized.get("method").unwrap_or(&noMethod).as_str().unwrap();
    msg.method = method.to_string();
    msg.req = param.to_string();
    msg.code = rHeads[1].to_string().parse::<u32>().unwrap_or(200);
    msg.bandwidth = band.parse::<u32>().unwrap_or(0);
    msg.start = start;
    msg.end = end;

    msg
}