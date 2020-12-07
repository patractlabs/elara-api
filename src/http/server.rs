use std::io::Read;
use rocket::{Request, Route, Data};
use rocket::http::{Method};
use serde::{Serialize, Deserialize};
use rocket_contrib::json::{Json};
use rocket::handler::{self, Handler};
// use futures::executor::block_on;
use super::validator::Validator;
use super::request::*;
use super::curl::*;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use crate::mq::producer_smol::*;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use log::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct ReqMessage {
    pub protocol: String,
    pub header: String,
    pub ip: String,
    pub chain: String,
    pub pid: String,
    pub method: String,
    pub req: String,
    pub resp: String,
    pub code: String,
    pub bandwidth: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>
}
#[derive(Serialize, Deserialize, Debug)]
pub struct KafkaInfo {
    pub key: String,
    pub message: ReqMessage
}

#[derive(Clone)]
pub struct HttpServer {
    target: Arc<HashMap<String, String>>,
    validator: Arc<Mutex<Validator>>,
    client: RequestCurl,
    // sender: KafkaProducerSmol,
    // txCh: crossbeam_channel::Sender<(String, String)>
    txCh: MessageSender<(String, String)>
}

impl ReqMessage {
    pub fn new() -> Self {
        ReqMessage{
            protocol: String::new(),
            header: String::new(),
            ip: String::new(),
            chain: String::new(),
            pid: String::new(),
            method: String::new(),
            req: String::new(),
            resp: String::new(),
            code: String::new(),
            bandwidth: String::new(),
            start: Utc::now(),
            end: Utc::now()
        }
    }
}
impl HttpServer {
    pub fn new(target: Arc<HashMap<String, String>>, vali: Arc<Mutex<Validator>>, tx: MessageSender<(String, String)>) -> Self {
        HttpServer{target: target, validator: vali, client: RequestCurl, txCh: tx}
    }

    pub fn Start(self) {
        tokio::spawn(async move {
            rocket::ignite().mount("/", self).launch();
        });
    }

    fn SendMsg(&self, key: &str, info: &str) -> bool {
        if self.txCh.Send((key.to_string(), info.to_string())) {
            return true;
        }
        false
    }
}

impl Handler for HttpServer {
    fn handle<'r>(&self, req: &'r Request, data: Data) -> handler::Outcome<'r> {
        let mut chain:String = String::new();
        let mut pid:String = String::new();
        if let Ok(achain) = req.get_param::<'r, String>(0).unwrap() {
            info!("{:?}", achain);
            chain = achain;
        }
        if let Ok(apid) = req.get_param::<'r, String>(1).unwrap(){
            info!("{:?}", apid);
            pid = apid;
        }
        let checker = self.validator.clone();
        // verify chain and project
        {
            let verifier = checker.lock().unwrap();
            let path = "/".to_string()+&chain+"/"+&pid;
            if !verifier.CheckLimit(path) {
                error!("project not exist");
                return handler::Outcome::from(req, Json(ApiResp{code:-1, message:"chain or project not match".to_string(), data:None}));
            }
        }
        if !self.target.contains_key(&chain) {
            debug!("chain {} not configured", chain);
            return handler::Outcome::from(req, Json(ApiResp{code:-3, message:"chain not config".to_string(), data:None}));
        }
        let chainAddr = self.target[&chain].clone();
        let mut contents = String::new();
        if let Err(e) = data.open().read_to_string(&mut contents) {
            return handler::Outcome::from(req, Json(ApiResp{code:-2, message:format!("{:?}", e), data:None}));
        }
        debug!("content:{}", contents);

        let start = Utc::now();
        let (resp, ok) = self.client.Rpc(&chainAddr, contents.clone());
        if ok {
            let end = Utc::now();
            let msg = KafkaInfo{key: "request".to_string(), message:parseRequest(req, &resp.data.unwrap_or("null".to_string()), &chain, &pid, &contents, start, end)};
            let info = serde_json::to_string(&msg).unwrap();
            // todo: async, do send in other thread
            // block_on(self.sender.sendMsg("api", &info));
            self.SendMsg("api", &info);
            return handler::Outcome::from(req, resp.mssage);
        }

        // todo: kafka
        handler::Outcome::from(req, Json(ApiResp{code:-4, message:"no response".to_string(), data:None}))
    }
}

impl Into<Vec<Route>> for HttpServer {
    fn into(self) -> Vec<Route> {
        vec![Route::new(Method::Post, "/<chain>/<pid>", self)]
    }
}

fn parseRequest(req: &Request, resp: &str, chain: &str, pid: &str, param: &str, start: DateTime<Utc>, end: DateTime<Utc>) -> ReqMessage {
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
    
    // println!("accept:{:?}, req head:{:?} heads: [{}] {:?}, {:?}", req.accept(), req.headers(), sliceHead.len(), sliceHead, rHeads);
    let mut msg = ReqMessage::new();
    msg.protocol = rHeads[0].to_string(); //rocket cannot get request protocol
    msg.ip = format!("{:?}",req.client_ip().unwrap_or(IpAddr::V4(Ipv4Addr::new(127, 127, 127, 127))));
    msg.header = format!("{:?}", req.headers());
    msg.chain = chain.to_string();
    msg.pid = pid.to_string();
    msg.method = req.method().as_str().to_string();
    msg.req = param.to_string();
    msg.code = rHeads[1].to_string();
    msg.bandwidth = band;
    msg.start = start;
    msg.end = end;
    // println!("{:?}", msg);
    msg
}

pub struct Broadcaster {
    sender: KafkaProducerSmol,
    rxCh: crossbeam_channel::Receiver<(String, String)>
}

impl Broadcaster {
    pub fn new(broker: String, topic: String, rx: crossbeam_channel::Receiver<(String, String)>) -> Self {
        Broadcaster{sender: KafkaProducerSmol::new(broker, topic), rxCh: rx}
    }

    fn MsgNotify(self) {
        let rpipe = self.rxCh;
        let kafka = self.sender;
        tokio::spawn(async move {
            loop {
                if let Ok(note) = rpipe.recv() {
                    kafka.sendMsg(&note.0, &note.1).await;
                }
            }
        });
    }

    pub fn Start(self) {
        self.MsgNotify();
    }
}

#[derive(Clone)]
pub struct MessageSender<T> {
    tx: crossbeam_channel::Sender<T>,
}

impl <T> MessageSender<T> {
    pub fn new(t: crossbeam_channel::Sender<T>) -> Self {
        MessageSender{tx: t}
    }

    pub fn Send(&self, val: T) -> bool {
        if let Err(e) = self.tx.send(val) {
            error!("send err: {}", e);
            return false;
        }
        true
    }
}