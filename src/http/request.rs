use serde::{Serialize, Deserialize};

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