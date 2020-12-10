// use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
// use tungstenite::accept_hdr;
use tungstenite::handshake::server::{Request, Response};
// use tungstenite::{connect};
use url::Url;

// use tungstenite::protocol::WebSocket;
// use tungstenite::client::AutoStream;
// use tungstenite::protocol::Message;

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use crate::http::validator::Validator;
use log::*;
use crate::http::server::{MessageSender, ReqMessage, KafkaInfo};
use chrono::{Utc};
use tokio_tungstenite::{WebSocketStream, connect_async, accept_hdr_async};
use tokio::net::{TcpListener, TcpStream};
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};
use futures_channel::mpsc::{unbounded};

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

    async fn Connect(target: &str) -> Box<WebSocketStream<TcpStream>> {
        // connect real target
        info!("connect websocket server: {}", target);
        let (socket, response) =
            connect_async(Url::parse(target).unwrap()).await.expect("Can't connect");

        info!("Connected to the server");
        info!("Response HTTP code: {}", response.status());
        info!("Response contains the following headers:");
        for (ref header, _value) in response.headers() {
            info!("* {}", header);
        }

        Box::new(socket)
    }

    pub async fn Start(& self) {
        info!("websocket listening {}", self.url);
        let server = TcpListener::bind(&self.url[..]).await.unwrap();
        while let Ok((stream, addr)) = server.accept().await {    
            debug!("guest {} coming", addr);
            
            let checker = self.validator.clone();
            let chains = self.target.clone();
            let caster = self.txCh.clone();
            tokio::spawn(async move {
            // std::thread::spawn( move || {
                let mut path = String::new();
                let clientIp = format!("{}", addr);
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
                let mut websocket = accept_hdr_async(stream, callback).await.unwrap();
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
                let targetSocket = WSProxy::Connect(&chains[pathSeg[1]]).await;
                let (svrHandleWr, svrHandleRd) = targetSocket.split();
                let (clientHandleWr, clientHandleRd) = websocket.split();

                let (txS2C, rxS2C) = unbounded();
                let (txC2S, rxC2S) = unbounded();

                let svrIncomingHandler = svrHandleRd.try_for_each(move |msg| {
                    if msg.is_binary() || msg.is_text() {
                        debug!("proxy rcv server: {}", msg);
                        txS2C.unbounded_send(msg.clone()).unwrap();

                        
                    } else if msg.is_close() {
                        debug!("server close {}", msg);
                        // return future::err(()); // compile error
                    }
                    future::ok(())
                });
                let SendClientHandler = rxS2C.map(Ok).forward(clientHandleWr);

                let clientIncomingHandler = clientHandleRd.try_for_each( |msg| {
                    let req = format!("{}", msg);
                    if msg.is_binary() || msg.is_text() {
                        debug!("proxy rcv client: {}", msg);
                        txC2S.unbounded_send(msg.clone()).unwrap();

                    } else if msg.is_close() {
                        debug!("client close {}", msg);
                        // return future::err(()); // compile error
                    }
                    let mut reqMsg = newReqMessage(&msgTemp);
                    reqMsg.req = req;
                    reqMsg.start = Utc::now();
                    reqMsg.code = "200".to_string();
                    let msg = KafkaInfo{key: "request".to_string(), message:reqMsg};
                    let info = serde_json::to_string(&msg).unwrap();
                    caster.Send(("api".to_string(), info));

                    future::ok(())
                });
                let SendSvrHandler = rxC2S.map(Ok).forward(svrHandleWr);

                tokio::spawn( async move {
                    pin_mut!(svrIncomingHandler, SendClientHandler);
                    future::select(svrIncomingHandler, SendClientHandler).await;
                });
                pin_mut!(clientIncomingHandler, SendSvrHandler);
                future::select(clientIncomingHandler, SendSvrHandler).await;
                info!("exit the link");
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