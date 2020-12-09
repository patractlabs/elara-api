use curl::easy::{Easy,List};
use std::io::Read;
use std::time::Duration;
use super::request::*;
use log::*;

#[derive(Clone)]
pub struct RequestCurl;

impl RequestCurl {
    pub fn Get(&self, url: &str) -> (BodyResp, bool) {
        let mut ret = BodyResp{code: -1, mssage: String::new(), data: None};
        let mut buf = Vec::new();
        let mut handle = Easy::new();
        handle.timeout(Duration::from_millis(4000));
        if let Err(e) = handle.url(url) {
            ret.mssage = format!("{:?}", e);
            return (ret, false);
        }
        {
            let mut transfer = handle.transfer();
            if let Err(e) = transfer.write_function(|data| {
                buf.extend_from_slice(data);
                // println!("{:?}", buf);
                Ok(data.len())
            }) {
                ret.mssage = format!("{:?}", e);
                return (ret, false);
            }
            if let Err(e) = transfer.perform() {
                error!("error: {}", e);
                ret.mssage = format!("{:?}", e);
                return (ret, false);
            }
        }
        let cache = String::from_utf8_lossy(&buf).to_string();
        debug!("rcv: {}", cache);
        if let Ok(obj) = serde_json::from_str::<ApiResp>(&cache) {
            ret.code = obj.code;
            ret.mssage = obj.message;
            return (ret, true);
        }

        return (ret, false);
    }

    pub fn Rpc(&self, url: &str, params: String) -> (BodyResp, bool) {
        let mut ret = BodyResp{code: -1, mssage: String::new(), data: None};
        let mut buf = Vec::new();
        let mut head = Vec::new();
    
        let mut handle = Easy::new();
        handle.timeout(Duration::from_millis(4000));
        if let Err(e) = handle.url(url) {
            ret.mssage = format!("{:?}", e);
            return (ret, false);
        }
        if let Err(e) = handle.post(true) {
            ret.mssage = format!("{:?}", e);
            return (ret, false);
        }
        if let Err(e) = handle.post_field_size(params.len() as u64) {
            ret.mssage = format!("{:?}", e);
            return (ret, false);
        }
        let mut list = List::new();
        if let Err(e) = list.append("Content-Type:application/json") {
            ret.mssage = format!("{:?}", e);
            return (ret, false);
        }
        if let Err(e) = handle.http_headers(list) {
            error!("error: {}", e);
            ret.mssage = format!("{:?}", e);
            return (ret, false);
        }
        {
            let mut transfer = handle.transfer();
            if let Err(e) = transfer.read_function(|buf| {
                Ok(params.as_bytes().read(buf).unwrap_or(0))
            }) {
                ret.mssage = format!("{:?}", e);
                return (ret, false);
            }

            if let Err(e) = transfer.write_function(|data| {
                buf.extend_from_slice(data);
                Ok(data.len())
            }) {
                ret.mssage = format!("{:?}", e);
                return (ret, false);
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

            if let Err(e) = transfer.perform() {
                error!("error: {}", e);
                ret.mssage = format!("{:?}", e);
                return (ret, false);
            }
        }
        
        ret.mssage = String::from_utf8_lossy(&buf).to_string();
        ret.data = Some(String::from_utf8_lossy(&head).to_string());
        debug!("rcv: {} {:?}", ret.mssage, ret.data);
        ret.code = 0;
        return (ret, true);

    }
}

#[derive(Clone)]
pub struct RequestMock;

impl RequestMock {
    pub fn Get(&self, url: &str) -> (BodyResp, bool) {
        let ret = BodyResp{code: 0, mssage: String::new(), data: None};
        return (ret, true);
    }

    pub fn Rpc(&self, url: &str, params: String) -> (BodyResp, bool) {
        let head = "HTTP/1.1 200 OK\r\nContent-Length: 1088\r\nContent-Type: application/json; charset=utf-8\r\nDate: Wed, 09 Dec 2020 07:40:22 GMT\r\nServer: Caddy\r\n";
        let ret = BodyResp{code: 0, mssage: "mock".to_string(), data: Some(head.to_string())};
        return (ret, true);
    }
}