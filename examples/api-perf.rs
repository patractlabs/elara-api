#![allow(unused_must_use)]
#![allow(non_snake_case)]
#![allow(unused_variables)]
use tungstenite::{connect, Message};
use url::Url;

use chrono::{Utc, Duration};

use std::io::Read;
use curl::easy::{Easy,List};
use std::thread;

trait Work {
    fn job(&self);
}

struct Websocket;
impl Work for Websocket {
fn job(&self) {

    let (mut socket, _response) =
        // elara api websocket server url: ip:port/chain/projectid
        connect(Url::parse("ws://8.210.110.126:7004/Polkadot/2754a6ab77be92d8338e9d0ed2f63585").unwrap()).expect("Can't connect");
        // connect(Url::parse("ws://8.210.110.126:7003/Polkadot/2754a6ab77be92d8338e9d0ed2f63585").unwrap()).expect("Can't connect");

    // println!("Connected to the server");
    // println!("Response HTTP code: {}", response.status());
    // println!("Response contains the following headers:");
    // for (ref header, _value) in response.headers() {
    //     println!("* {}", header);
    // }

    socket
        .write_message(Message::Text("{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"chain_getBlock\",\"params\":[]}".into()))
        .unwrap();
    
        let msg = socket.read_message().expect("Error reading message");
        // println!("Received: {}", msg);

        socket.close(None);
 
}
}

fn measure(name: String, no: i32, obj: &dyn Work, cnt: i32) {
    let mut min = Duration::seconds(100);
    let mut max = min - min;
    let mut sum = min - min;

    for _i in 1..cnt {
        let start = Utc::now();
        obj.job();
        let end = Utc::now();
        let delta = end - start;
        if delta > max {
            max = delta;
        }
        if delta < min {
            min = delta;
        }

        sum = sum + delta;
    }

    println!("{}[{}] max: {}, min: {}, avg: {}", name, no, max, min, sum/cnt);
}

fn main() {
    let mut holds = Vec::new();
    for i in 1..10 {
        let j = i;
        let handle = thread::spawn(move ||{
            let ws = Websocket;
            measure("websocket".to_string(), j, &ws, 10);
        });
        holds.push(handle);
    }
    for h in holds {
        h.join().unwrap();
    }

    // let hs = HttpRequest;
    // measure(&hs, 10);
}

struct HttpRequest;

impl HttpRequest {
    pub fn Rpc(&self, url: &str, params: String) -> bool {
        let mut buf = Vec::new();
        let mut head = Vec::new();
    
        let mut handle = Easy::new();
        handle.timeout(std::time::Duration::from_millis(4000));
        if let Err(_e) = handle.url(url) {
            return false;
        }
        if let Err(_e) = handle.post(true) {
            return false;
        }
        if let Err(_e) = handle.post_field_size(params.len() as u64) {
            return false;
        }
        let mut list = List::new();
        if let Err(_e) = list.append("Content-Type:application/json") {
            return false;
        }
        if let Err(_e) = handle.http_headers(list) {
            // println!("error: {}", e);
            return false;
        }
        {
            let mut transfer = handle.transfer();
            if let Err(_e) = transfer.read_function(|buf| {
                Ok(params.as_bytes().read(buf).unwrap_or(0))
            }) {
                return false;
            }

            if let Err(_e) = transfer.write_function(|data| {
                buf.extend_from_slice(data);
                Ok(data.len())
            }) {
                return false;
            }

            transfer.header_function(|data| {
                // println!("header: {}", String::from_utf8_lossy(data));
                head.extend_from_slice(data);
                true
            }).unwrap();
            // if let Err(e) = transfer.header_function(|data| {
            //     println!("header: {}", String::from_utf8_lossy(data));
            //     true
            // }) {
            //     ret.mssage = format!("{:?}", e);
            //     return (ret, false);
            // } //segment fault

            if let Err(_e) = transfer.perform() {
                // println!("error: {}", e);
                return false;
            }
        }
        
        // println!("rcv: {} {}", String::from_utf8_lossy(&buf).to_string(), String::from_utf8_lossy(&head).to_string());
        return true;

    }
}
impl Work for HttpRequest {
    fn job(&self) {
        // self.Rpc("http://8.210.110.126:8300/Polkadot/2754a6ab77be92d8338e9d0ed2f63585", "{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"chain_getBlock\",\"params\":[]}".to_string());
        self.Rpc("http://8.210.110.126:7003/Polkadot/2754a6ab77be92d8338e9d0ed2f63585", "{\"id\":1,\"jsonrpc\":\"2.0\",\"method\":\"chain_getBlock\",\"params\":[]}".to_string());
    }
}