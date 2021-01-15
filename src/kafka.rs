use rdkafka::config::ClientConfig;
use rdkafka::consumer::stream_consumer::StreamConsumer;

use rdkafka::consumer::{ConsumerContext, Rebalance};
use rdkafka::topic_partition_list::TopicPartitionList;
use rdkafka::ClientContext;

use log::info;
use rdkafka::util::DefaultRuntime;

use futures::{Stream, TryStreamExt};
pub use rdkafka::config::RDKafkaLogLevel as LogLevel;
pub use rdkafka::consumer::Consumer;
pub use rdkafka::error::KafkaError;
pub use rdkafka::error::KafkaResult;
pub use rdkafka::message::BorrowedMessage;
use rdkafka::message::OwnedMessage;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::broadcast::channel;
use tokio::sync::broadcast::{Receiver, Sender};

struct KVContext;

impl ClientContext for KVContext {}

// TODO: impl some useful logics
impl ConsumerContext for KVContext {
    fn pre_rebalance(&self, rebalance: &Rebalance) {
        info!("Pre rebalance {:?}", rebalance);
    }

    fn post_rebalance(&self, rebalance: &Rebalance) {
        info!("Post rebalance {:?}", rebalance);
    }

    fn commit_callback(&self, result: KafkaResult<()>, _offsets: &TopicPartitionList) {
        info!("Committing offsets: {:?}", result);
    }
}

/// KvConsumer is shared with all ws connections
pub struct KvConsumer(StreamConsumer<KVContext>);

// trait KafkaMessageStream: Stream<Item=KafkaResult<OwnedMessage>> + {
//
// }

// pub type KafkaMessageStream<'a> = MessageStream<'a, KVContext>;

impl KvConsumer {
    pub fn new<T>(config: T, level: LogLevel) -> Self
    where
        T: Iterator<Item = (String, String)>,
    {
        let context = KVContext {};

        // TODO: arrange config
        let mut cfg = ClientConfig::new();

        for (k, v) in config.into_iter() {
            cfg.set(k.as_str(), v.as_str());
        }
        cfg.set_log_level(level);

        let consumer: StreamConsumer<KVContext> = cfg
            .create_with_context(context)
            .expect("Consumer creation failed");

        Self(consumer)
    }

    pub fn subscribe(&self, topics: &[&str]) -> KafkaResult<()> {
        self.0.subscribe(topics)
    }

    pub fn stream(&self) -> impl Stream<Item = KafkaResult<OwnedMessage>> + '_ {
        self.0
            .stream()
            .map_ok(|borrowed_message| borrowed_message.detach())
    }

    pub async fn recv(&self) -> Result<OwnedMessage, KafkaError> {
        self.0.recv().await.map(|msg| msg.detach())
    }
}

pub struct KVSubscriber {
    // TODO: support consumers
    consumer: KvConsumer,
    sender: Sender<OwnedMessage>,
}

impl KVSubscriber {
    pub fn new(consumer: KvConsumer, chan_size: usize) -> Self {
        let (sender, receiver) = broadcast::channel(chan_size);
        Self { consumer, sender }
    }

    /// start to subscribe kafka data.
    pub async fn start(&self) {
        loop {
            let msg = self.consumer.recv().await;
            match msg {
                Ok(msg) => {
                    self.sender.send(msg);
                }
                Err(err) => {
                    log::error!("{}", err);
                }
            };
        }
    }

    /// Creates a new [`Receiver`] handle that will receive values sent **after**
    /// this call to `subscribe`.
    pub fn subscribe(&mut self) -> Receiver<OwnedMessage> {
        self.sender.subscribe()
    }
}
