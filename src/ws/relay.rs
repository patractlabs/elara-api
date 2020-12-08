use std::net::{TcpListener, Ipv4Addr, SocketAddr, SocketAddrV4};
use tungstenite::accept_hdr;
use tungstenite::handshake::server::{Request, Response};
use tungstenite::{connect};
use url::Url;

use tungstenite::protocol::WebSocket;
use tungstenite::client::AutoStream;

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use crate::http::validator::Validator;
use log::*;
use crate::http::server::{MessageSender, ReqMessage, KafkaInfo};
use chrono::{Utc};

pub struct WSProxy {
    url: String,
    target: Arc<HashMap<String, String>>,
    validator: Arc<Mutex<Validator>>,
    txCh: MessageSender<(String, String)>
}

impl WSProxy {
    pub fn new(url: String, target: Arc<HashMap<String, String>>, vali: Arc<Mutex<Validator>>, tx: MessageSender<(String, String)>) -> Self {
        WSProxy{url: url, target: target, validator: vali, txCh: tx}
    }

    fn Connect(target: &str) -> Box<WebSocket<AutoStream>> {
        // connect real target
        info!("connect websocket server: {}", target);
        let (socket, response) =
            connect(Url::parse(target).unwrap()).expect("Can't connect");

        info!("Connected to the server");
        info!("Response HTTP code: {}", response.status());
        info!("Response contains the following headers:");
        for (ref header, _value) in response.headers() {
            info!("* {}", header);
        }

        Box::new(socket)
    }

    pub fn Start(& self) {
        info!("websocket listening {}", self.url);
        let server = TcpListener::bind(&self.url[..]).unwrap();
        for link in server.incoming() {
            debug!("guest coming");
            let stream = match link {
                Ok(conn) => conn,
                Err(e) => {
                    error!("link in error: {}", e);
                    continue;
                }
            };
            let checker = self.validator.clone();
            let chains = self.target.clone();
            let caster = self.txCh.clone();
            // tokio::spawn(async move {
            std::thread::spawn( move || {
                let mut path = String::new();
                let clientIp = format!("{}", stream.peer_addr().unwrap_or(SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 127, 127, 127), 1111))));
                info!("client ip: {}", clientIp);
                let mut msgTemp = ReqMessage::new();
                
                let callback = |req: &Request, mut response: Response| {
                    info!("Received a new ws handshake");
                    path = req.uri().path().to_string();
                    info!("The request's path is: {}", path);
                    info!("The request's headers are:");
                    for (ref header, _value) in req.headers() {
                        info!("* {}", header);
                    }

                    makeReqMessageTemplate(&mut msgTemp, req, clientIp);

                    // Let's add an additional header to our response to the client.
                    let headers = response.headers_mut();
                    headers.append("MyCustomHeader", ":)".parse().unwrap());
                    headers.append("SOME_TUNGSTENITE_HEADER", "header_value".parse().unwrap());

                    Ok(response)
                };
                let mut websocket = accept_hdr(stream, callback).unwrap();
                let pathStr = path.clone();
                let pathSeg = pathStr.split("/").collect::<Vec<&str>>();
                // verify chain and project
                {
                    let verifier = checker.lock().unwrap();
                    if !verifier.CheckLimit(path) {
                        error!("project not exist");
                        // targetSocket.close(None);
                        websocket.close(None);
                        return;
                    }
                }
                // connect server
                if !chains.contains_key(pathSeg[1]) {
                    println!("chain {} not configured", pathSeg[1]);
                    websocket.close(None);
                    return;
                }
                let mut targetSocket = WSProxy::Connect(&chains[pathSeg[1]]);
                
                // tokio::spawn(move {
                    loop {
                        if let Ok(msg) = websocket.read_message() {
                            let start = Utc::now();
                            let req = format!("{}", msg);
                            if msg.is_binary() || msg.is_text() {
                                debug!("proxy rcv client: {}", msg);
                                if let Err(e) = targetSocket.write_message(msg) {
                                    error!("write server error {}", e);
                                    break;
                                }

                                match targetSocket.read_message() {
                                    Ok(msg) => {
                                        if msg.is_binary() || msg.is_text() {
                                            debug!("proxy rcv server: {}", msg);
                                            if let Err(e) = websocket.write_message(msg) {
                                                error!("write client error {}", e);
                                                break;
                                            }
                                        } else if msg.is_close() {
                                            debug!("server close {}", msg);
                                            break;
                                        }
                                    },
                                    Err(e) => {
                                        error!("server error {}", e);
                                        break;
                                    }

                                }
                            } else if msg.is_close() {
                                debug!("client close {}", msg);
                                break;
                            }
                            let end = Utc::now();
                            let mut reqMsg = newReqMessage(&msgTemp);
                            reqMsg.req = req;
                            reqMsg.start = start;
                            reqMsg.end = end;
                            reqMsg.code = "200".to_string();
                            let msg = KafkaInfo{key: "request".to_string(), message:reqMsg};
                            let info = serde_json::to_string(&msg).unwrap();
                            caster.Send(("api".to_string(), info));
                        } else {
                            info!("client lost");
                            break;
                        }
                        
                    }
                    info!("exit the link");
                    targetSocket.close(None);
                    websocket.close(None);
                // });
            });
        }
    }
}

fn makeReqMessageTemplate(msg: &mut ReqMessage, req: &Request, client: String) {
    msg.protocol = "websocket".to_string();
    msg.ip = client;
    let path = req.uri().path().to_string();
    let pathSeg = path.split("/").collect::<Vec<&str>>();
    msg.chain = pathSeg[1].to_string();
    msg.pid = pathSeg[2].to_string();
    msg.method = req.method().as_str().to_string();
    msg.header = format!("{:?}", req.headers());
}

fn newReqMessage(temp: &ReqMessage) -> ReqMessage {
    let mut msg = ReqMessage::new();
    msg.protocol = temp.protocol.clone();
    msg.ip = temp.ip.clone();
    msg.chain = temp.chain.clone();
    msg.pid = temp.pid.clone();
    msg.method = temp.method.clone();
    msg.header = temp.header.clone();
    msg
}