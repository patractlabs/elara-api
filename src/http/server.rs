use serde::{Serialize, Deserialize};
use crate::mq::producer_smol::*;
use log::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct ReqMessage {
    pub protocol: String,
    pub header: String,
    pub ip: String,
    pub chain: String,
    pub pid: String,
    pub method: String,
    pub req: String,
    pub resp: String,
    pub code: u32,
    pub bandwidth: u32,
    pub start: i64,
    pub end: i64
}
#[derive(Serialize, Deserialize, Debug)]
pub struct KafkaInfo {
    pub key: String,
    pub message: ReqMessage
}

impl ReqMessage {
    pub fn new() -> Self {
        ReqMessage{
            protocol: String::new(),
            header: String::new(),
            ip: String::new(),
            chain: String::new(),
            pid: String::new(),
            method: String::new(),
            req: String::new(),
            resp: String::new(),
            code: 0,
            bandwidth: 0,
            start: 0,
            end: 0
        }
    }
}

pub struct Broadcaster {
    sender: KafkaProducerSmol,
    rxCh: crossbeam_channel::Receiver<(String, String)>
}

impl Broadcaster {
    pub fn new(broker: String, topic: String, rx: crossbeam_channel::Receiver<(String, String)>) -> Self {
        Broadcaster{sender: KafkaProducerSmol::new(broker, topic), rxCh: rx}
    }

    fn MsgNotify(self) {
        let rpipe = self.rxCh;
        let kafka = self.sender;
        // tokio::spawn(async move {
        std::thread::spawn( move || {
            loop {
                if let Ok(note) = rpipe.recv() {
                    futures::executor::block_on(kafka.sendMsg(&note.0, &note.1));
                }
            }
        });
    }

    pub fn Start(self) {
        self.MsgNotify();
    }
}

#[derive(Clone)]
pub struct MessageSender<T> {
    tx: crossbeam_channel::Sender<T>,
}

impl <T> MessageSender<T> {
    pub fn new(t: crossbeam_channel::Sender<T>) -> Self {
        MessageSender{tx: t}
    }

    pub fn Send(&self, val: T) -> bool {
        if let Err(e) = self.tx.send(val) {
            error!("send err: {}", e);
            return false;
        }
        true
    }
}

pub fn parseIp(forward: &str, defaultIp: String) -> String {
    let ips = forward.split(", ").collect::<Vec<&str>>();
    if ips[0] != "" {
        return ips[0].to_string();
    }
    defaultIp
}