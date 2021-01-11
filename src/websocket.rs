use actix::{Actor, StreamHandler};
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;
use rustls::{
    internal::pemfile::{certs, pkcs8_private_keys},
    AllowAnyAnonymousOrAuthenticatedClient, Certificate, RootCertStore, ServerConfig, Session,
};

use std::{any::Any, env, fs::File, io::BufReader, net::SocketAddr};

const CA_CERT: &str = "certs/rootCA.pem";
const SERVER_CERT: &str = "certs/server-cert.pem";
const SERVER_KEY: &str = "certs/server-key.pem";

/// Define HTTP actor
struct MyWs;

impl Actor for MyWs {
    type Context = ws::WebsocketContext<Self>;
}

/// Handler for ws::Message message
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for MyWs {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            Ok(ws::Message::Text(text)) => ctx.text(text),
            Ok(ws::Message::Binary(bin)) => ctx.binary(bin),
            _ => (),
        }
    }
}

async fn index(req: HttpRequest, stream: web::Payload) -> Result<HttpResponse, Error> {
    let resp = ws::start(MyWs {}, &req, stream);
    println!("{:?}", resp);
    resp
}

pub async fn main() -> std::io::Result<()> {
    let mut cert_store = RootCertStore::empty();

    // import CA cert
    let ca_cert = &mut BufReader::new(File::open(CA_CERT)?);
    cert_store
        .add_pem_file(ca_cert)
        .expect("root CA not added to store");
    // set up client authentication requirements
    let client_auth = AllowAnyAnonymousOrAuthenticatedClient::new(cert_store);
    let config = ServerConfig::new(client_auth);

    let res = HttpServer::new(|| App::new().route("/ws/", web::get().to(index)))
        .bind_rustls(("localhost", 8443), config)?
        // .bind("127.0.0.1:8080")?
        .run()
        .await;

    println!("{:?}", res);

    res
}
