use std::collections::HashMap;
use super::validator::Validator;
use super::request::*;
use super::curl::*;
use chrono::{DateTime, Utc};
use log::*;
use actix_web::{get, post, middleware, web, App, HttpRequest, HttpResponse, HttpServer};
use std::sync::{Arc, Mutex};
use crate::http::server::{MessageSender, ReqMessage, KafkaInfo};

#[derive(Clone, Copy)]
pub struct ActixWebServer<'a> {
    target: &'a Arc<HashMap<String, String>>,
    validator: &'a Validator,
    client: RequestCurl,
    txCh: &'a MessageSender<(String, String)>
}

impl <'a> ActixWebServer<'a> {
    pub fn new(target: &'a Arc<HashMap<String, String>>, vali: &'a Validator, tx: &'a MessageSender<(String, String)>) -> Self {
        ActixWebServer{target: target, validator: vali, client: RequestCurl, txCh: tx}
    }

    pub async fn doJob(&self, req: HttpRequest, web::Path((chain, pid)): web::Path<(String, String)>, mut body: web::Payload) -> &'static str {
        "Hello job!\r\n"
    }
}

pub async fn RunServer(svr: ActixWebServer<'static>, port: String) -> std::io::Result<()> {
        HttpServer::new(move || {
            App::new()
                .wrap(middleware::DefaultHeaders::new().header("X-Version", "0.2"))
                .wrap(middleware::Compress::default())
                .wrap(middleware::Logger::default())
                .service(index)
                .service(web::resource("/{chain}/{pid}").route(web::post().to(async move |req: HttpRequest, path: web::Path<(String, String)>, mut body: web::Payload| {svr.doJob(req, path, body).await})))
        })
        .bind(format!("0.0.0.0:{}", port))?
        .run()
        .await
}

#[get("/actix")]
async fn index() -> &'static str {
    "Hello world!\r\n"
}