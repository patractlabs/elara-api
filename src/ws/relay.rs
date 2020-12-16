use tungstenite::handshake::server::{Request, Response};
use url::Url;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use crate::http::validator::Validator;
use log::*;
use crate::http::server::{MessageSender, ReqMessage};
use chrono::{Utc};
use async_tungstenite::{WebSocketStream, accept_hdr_async};
use async_tungstenite::async_std::connect_async;
use async_std::net::{TcpListener, TcpStream};
use async_std::task;
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};
use futures_channel::mpsc::{unbounded};
use serde_json::{Value};
use http::header::HeaderMap;

pub struct WSProxy {
    url: String,
    target: Arc<HashMap<String, String>>,
    validator: Validator,
    txCh: MessageSender<(String, String)>
}

impl WSProxy {
    pub fn new(url: String, target: Arc<HashMap<String, String>>, vali: Validator, tx: MessageSender<(String, String)>) -> Self {
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
            task::spawn(async move {
                let mut path = String::new();
                let mut clientIp = format!("{}", addr);
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
                    if let Some(forwards) = req.headers().get("x-forwarded-for") {
                        if let Ok(ip) = forwards.to_str() {
                            clientIp = ip.to_string();
                        }
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
                    let verifier = checker;
                    if !verifier.CheckLimit(path) {
                        error!("project not exist");
                        websocket.close(None);
                        return;
                    }
                }
                // connect server
                if !chains.contains_key(pathSeg[1]) {
                    error!("chain {} not configured", pathSeg[1]);
                    websocket.close(None);
                    return;
                }
                let targetSocket = WSProxy::Connect(&chains[pathSeg[1]]).await;
                let (svrHandleWr, svrHandleRd) = targetSocket.split();
                let (clientHandleWr, clientHandleRd) = websocket.split();

                let (txS2C, rxS2C) = unbounded();
                let (txC2S, rxC2S) = unbounded();

                let gMethod = Arc::new(Mutex::new(String::new()));

                let defaultMethod = gMethod.clone();
                let svrIncomingHandler = svrHandleRd.try_for_each(move |msg| {
                    if msg.is_binary() || msg.is_text() {
                        debug!("proxy rcv server: {}", msg);
                        txS2C.unbounded_send(msg.clone()).unwrap();
                        let contents = msg.into_text().unwrap();
                        let deserialized: Value = serde_json::from_str(&contents).unwrap();
                        let mut reqMsg = newReqMessage(&msgTemp);
                        if let Some(value) = deserialized.get("method") {
                            if let Some(method) = value.as_str() {
                                reqMsg.method = method.to_string();
                            }
                        } else {
                            reqMsg.method = defaultMethod.lock().unwrap().to_string();
                            warn!("proxy rcv-method server: {}; default method: {}", contents, reqMsg.method);
                        }
                        reqMsg.bandwidth = contents.len().to_string().parse::<u32>().unwrap();
                        reqMsg.start = Utc::now().timestamp_millis();
                        reqMsg.end = reqMsg.start;
                        reqMsg.code = 200;
                        let info = serde_json::to_string(&reqMsg).unwrap();
                        caster.Send(("request".to_string(), info));
                    } else if msg.is_close() {
                        debug!("server close {}", msg);
                    }
                    future::ok(())
                });
                let SendClientHandler = rxS2C.map(Ok).forward(clientHandleWr);

                let cacheMethod = gMethod.clone();
                let clientIncomingHandler = clientHandleRd.try_for_each( |msg| {
                    let req = format!("{}", msg);
                    if msg.is_binary() || msg.is_text() {
                        debug!("proxy rcv client: {}", msg);
                        txC2S.unbounded_send(msg.clone()).unwrap();
                        let contents = msg.into_text().unwrap();
                        let deserialized: Value = serde_json::from_str(&contents).unwrap();
                        if let Some(value) = deserialized.get("method") {
                            if let Some(method) = value.as_str() {
                                let mut cm = cacheMethod.lock().unwrap();
                                *cm = method.to_string();
                            }
                        }
                    } else if msg.is_close() {
                        debug!("client close {}", msg);
                    }

                    future::ok(())
                });
                let SendSvrHandler = rxC2S.map(Ok).forward(svrHandleWr);

                task::spawn( async move {
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

pub fn RunWebSocketBg(proxy: WSProxy) {
    task::spawn(async move {
        proxy.Start().await;
    });
}

fn makeReqMessageTemplate(msg: &mut ReqMessage, req: &Request, client: String) {
    msg.protocol = "websocket".to_string();
    msg.ip = client;
    let path = req.uri().path().to_string();
    let pathSeg = path.split("/").collect::<Vec<&str>>();
    msg.chain = pathSeg[1].to_string();
    msg.pid = pathSeg[2].to_string();
    msg.header = headerString(req.headers());
}

fn newReqMessage(temp: &ReqMessage) -> ReqMessage {
    let mut msg = ReqMessage::new();
    msg.protocol = temp.protocol.clone();
    msg.ip = temp.ip.clone();
    msg.chain = temp.chain.clone();
    msg.pid = temp.pid.clone();
    msg.method = String::new();
    msg.header = temp.header.clone();
    msg
}

fn headerString(h: &HeaderMap) -> HashMap<String, String> {
    let mut container = HashMap::new();
    for (key, value) in h.iter() {
        let k = key.as_str().to_string();
        let v = value.to_str().unwrap().to_string();
        container.insert(k, v);
    }
    container
}