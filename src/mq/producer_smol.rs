use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::util::AsyncRuntime;
use std::future::Future;
use futures::future::{self, FutureExt};
use std::time::{Duration, Instant};

pub struct SmolRuntime;

impl AsyncRuntime for SmolRuntime {
    type Delay = future::Map<smol::Timer, fn(Instant)>;

    fn spawn<T>(task: T)
    where
        T: Future<Output = ()> + Send + 'static,
    {
        smol::spawn(task).detach()
    }

    fn delay_for(duration: Duration) -> Self::Delay {
        smol::Timer::after(duration).map(|_| ())
    }
}

#[derive(Clone)]
pub struct KafkaProducerSmol {
    broker: String,
    topic: String,
    client: FutureProducer
}

impl KafkaProducerSmol {
    pub fn new(broker: String, topic: String) -> Self {
        let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &broker)
        .set("message.timeout.ms", "5000")
        .create()
        .expect("Producer creation error");

        KafkaProducerSmol{broker: broker, topic: topic, client: producer}
    }

    pub async fn sendMsg(&self, key: &str, msg: &str) -> bool {
        println!("send kafka {} {}", key, msg);
        let delivery_status = self.client
                .send_with_runtime::<SmolRuntime, str, _, _>(
                    FutureRecord::to(&self.topic)
                        .payload(msg)
                        .key(key),
                    Duration::from_secs(0),
                )
                .await;
        match delivery_status {
            Ok(status) => {
                //todo: log
                return true;
            },
            Err(e) => {
                //todo: log
                return false;
            }
        }
    }
}