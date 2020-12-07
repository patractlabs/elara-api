use reqwest::Client; 
use serde::{Serialize, Deserialize};
use std::time::Duration;
use log::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct RpcReq {
    id: i32,
    jsonrpc: String,
    method: String,
    params: Vec<String>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BodyResp {
    pub code: i32,
    pub mssage: String,
    pub data: Option<String>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ApiResp {
    pub code: i32,
    pub message: String,
    pub data: Option<String>
}

#[derive(Clone)]
pub struct HttpRequest {
    client: Client
}

impl HttpRequest {
    pub fn new() -> Self {
        let client = Client::builder().timeout(Duration::from_millis(20000)).build().expect("create http client fail");
        let hr = HttpRequest{client: client};
        hr
    }

    pub async fn GetSimple(&self, url: &str) -> (BodyResp, bool) {
        let client = &self.client;
        let resp = client.get(url).send().await;
        match resp {
            Ok(r) => {
                let body = r.json::<BodyResp>().await;
                match body {
                    Ok(b) => {
                        debug!("Got content:\n{:?}\n",  b);
                        return (b, true);
                    },
                    // Ok(b) => println!("Got {} bytes\nheader:\n{:?}\n", b.len(), b),
                    Err(e) => {
                        error!("Got an error: {}", e);
                        return (BodyResp{code:-1, mssage:"".to_string(), data:None}, false);
                    },
                }
            },
            Err(e) => {
                error!("Got an error: {}", e);
                return (BodyResp{code:-1, mssage:"".to_string(), data:None}, false);
            },
        }
        
    }

    pub async fn Get(&self, url: &str) -> (BodyResp, bool) {
        let client = &self.client;
        let resp = client.get(url).send().await;
        match resp {
            Ok(r) => {
                let body = r.json::<ApiResp>().await;
                match body {
                    Ok(b) => {
                        debug!("Got content:\n{:?}\n",  b);
                        return (BodyResp{code:b.code, mssage:b.message.to_owned(), data:None}, true);
                    },
                    // Ok(b) => println!("Got {} bytes\nheader:\n{:?}\n", b.len(), b),
                    Err(e) => {
                        error!("Got an error: {}", e);
                        return (BodyResp{code:-1, mssage:"".to_string(), data:None}, false);
                    },
                }
            },
            Err(e) => {
                error!("Got an error: {}", e);
                return (BodyResp{code:-1, mssage:"".to_string(), data:None}, false);
            },
        }
        
    }

    pub async fn PostSimple(&self, url: &str, params: String) -> (BodyResp, bool) {
        let client = &self.client;
        let resp = client.post(url).body(params).send().await;
        match resp {
            Ok(r) => {
                let body = r.json::<ApiResp>().await;
                match body {
                    Ok(b) => {
                        debug!("Got content:\n{:?}\n",  b);
                        return (BodyResp{code:b.code, mssage:b.message.to_owned(), data:None}, true);
                    },
                    // Ok(b) => println!("Got {} bytes\nheader:\n{:?}\n", b.len(), b),
                    Err(e) => {
                        debug!("Got an error: {}", e);
                        return (BodyResp{code:-1, mssage:"".to_string(), data:None}, false);
                    },
                }
            },
            Err(e) => {
                debug!("Got an error: {}", e);
                return (BodyResp{code:-1, mssage:"".to_string(), data:None}, false);
            },
        }
        
    }

    pub async fn Rpc(&self, url: &str, params: String) -> (BodyResp, bool) {
        let mut ret = BodyResp{code: -1, mssage: "".to_string(), data: None};
        let client = &self.client;
        let resp = client.post(url).header("Content-Type", "application/json").body(params).send().await;
        match resp {
            Ok(r) => {
                let body = r.bytes().await;
                match body {
                    Ok(b) => {
                        debug!("Got content:\n{:?}\n",  b);
                        ret.code = 0;
                        ret.mssage = String::from_utf8_lossy(&b).to_string();
                        return (ret, true);
                    },
                    // Ok(b) => println!("Got {} bytes\nheader:\n{:?}\n", b.len(), b),
                    Err(e) => {
                        error!("Got an error: {}", e);
                        return (ret, false);
                    },
                }
            },
            Err(e) => {
                error!("Got an error: {}", e);
                return (ret, false);
            },
        }
        
    }
}