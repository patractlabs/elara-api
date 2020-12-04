use std::net::TcpListener;
use tungstenite::accept_hdr;
use tungstenite::handshake::server::{Request, Response};
use tungstenite::{connect};
use url::Url;

use tungstenite::protocol::WebSocket;
use tungstenite::client::AutoStream;

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use crate::http::validator::Validator;

pub struct WSProxy {
    url: String,
    target: Arc<HashMap<String, String>>,
    validator: Arc<Mutex<Validator>>
}

impl WSProxy {
    pub fn new(url: String, target: Arc<HashMap<String, String>>, vali: Arc<Mutex<Validator>>) -> Self {
        WSProxy{url: url, target: target, validator: vali}
    }

    fn Connect(target: &str) -> Box<WebSocket<AutoStream>> {
        // connect real target
        println!("connect websocket server: {}", target);
        let (socket, response) =
            connect(Url::parse(target).unwrap()).expect("Can't connect");

        println!("Connected to the server");
        println!("Response HTTP code: {}", response.status());
        println!("Response contains the following headers:");
        for (ref header, _value) in response.headers() {
            println!("* {}", header);
        }

        Box::new(socket)
    }

    pub fn Start(& self) {
        println!("websocket listening {}", self.url);
        let server = TcpListener::bind(&self.url[..]).unwrap();
        for stream in server.incoming() {
            println!("guest coming");
            let checker = self.validator.clone();
            let chains = self.target.clone();
            tokio::spawn(async move {
                let mut path = String::new();
                let callback = |req: &Request, mut response: Response| {
                    println!("Received a new ws handshake");
                    path = req.uri().path().to_string();
                    println!("The request's path is: {}", path);
                    println!("The request's headers are:");
                    for (ref header, _value) in req.headers() {
                        println!("* {}", header);
                    }

                    // Let's add an additional header to our response to the client.
                    let headers = response.headers_mut();
                    headers.append("MyCustomHeader", ":)".parse().unwrap());
                    headers.append("SOME_TUNGSTENITE_HEADER", "header_value".parse().unwrap());

                    Ok(response)
                };
                let mut websocket = accept_hdr(stream.unwrap(), callback).unwrap();
                let pathStr = path.clone();
                let pathSeg = pathStr.split("/").collect::<Vec<&str>>();
                // verify chain and project
                {
                    let verifier = checker.lock().unwrap();
                    if !verifier.CheckLimit(path) {
                        println!("project not exist");
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
                            if msg.is_binary() || msg.is_text() {
                                println!("proxy rcv client: {}", msg);
                                if let Err(e) = targetSocket.write_message(msg) {
                                    println!("write server error {}", e);
                                    break;
                                }

                                match targetSocket.read_message() {
                                    Ok(msg) => {
                                        if msg.is_binary() || msg.is_text() {
                                            println!("proxy rcv server: {}", msg);
                                            if let Err(e) = websocket.write_message(msg) {
                                                println!("write client error {}", e);
                                                break;
                                            }
                                        } else if msg.is_close() {
                                            println!("server close {}", msg);
                                            break;
                                        }
                                    },
                                    Err(e) => {
                                        println!("server error {}", e);
                                        break;
                                    }

                                }
                            } else if msg.is_close() {
                                println!("client close {}", msg);
                                break;
                            }
                        } else {
                            println!("client lost");
                            break;
                        }
                        
                    }
                    println!("exit the link");
                    targetSocket.close(None);
                    websocket.close(None);
                // });
            });
        }
    }
}