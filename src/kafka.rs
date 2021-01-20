use rdkafka::config::ClientConfig;
pub use rdkafka::config::RDKafkaLogLevel as LogLevel;
use rdkafka::consumer::stream_consumer::StreamConsumer;
pub use rdkafka::consumer::Consumer;
use rdkafka::consumer::{ConsumerContext, Rebalance};
pub use rdkafka::error::KafkaError;
pub use rdkafka::error::KafkaResult;
pub use rdkafka::message::BorrowedMessage;
pub use rdkafka::message::OwnedMessage;
use rdkafka::topic_partition_list::TopicPartitionList;
use rdkafka::ClientContext;

use futures::{Stream, TryStreamExt};
use log::info;

use std::sync::Arc;
use tokio::sync::broadcast;
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

pub struct KvConsumer(StreamConsumer<KVContext>);

impl KvConsumer {
    /// Uses the current configuration to create a new Consumer
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

    /// Subscribes the consumer to a list of topics.
    pub fn subscribe(&self, topics: &[&str]) -> KafkaResult<()> {
        self.0.subscribe(topics)
    }

    /// Constructs a stream that yields messages from this consumer.
    pub fn stream(&self) -> impl Stream<Item = KafkaResult<OwnedMessage>> + '_ {
        self.0
            .stream()
            .map_ok(|borrowed_message| borrowed_message.detach())
    }

    /// Receives the next message from the stream.
    pub async fn recv(&self) -> Result<OwnedMessage, KafkaError> {
        self.0.recv().await.map(|msg| msg.detach())
    }
}

/// Dispatch kafka consumer's data to different receivers.
pub struct KVSubscriber {
    // TODO: support consumers
    consumer: Arc<KvConsumer>,
    sender: Sender<OwnedMessage>,
    _receiver: Receiver<OwnedMessage>,
}

impl KVSubscriber {
    /// wrap a kafka consumer to dispatch consumer data to multiple receivers
    pub fn new(consumer: Arc<KvConsumer>, chan_size: usize) -> Self {
        let (sender, _receiver) = broadcast::channel(chan_size);
        Self {
            sender,
            consumer,
            _receiver,
        }
    }

    /// start to subscribe kafka data.
    pub fn start(&self) {
        log::debug!("Started background kafka subscription");
        let consumer = self.consumer.clone();
        let sender = self.sender.clone();
        tokio::spawn(async move {
            loop {
                let msg = consumer.recv().await;
                log::debug!("{:?}", &msg);
                match msg {
                    Ok(msg) => {
                        if let Err(err) = sender.send(msg) {
                            log::warn!("error when send kafka data: {}", err);
                        }
                    }

                    Err(err) => {
                        log::error!("kafka error: {}", err);
                    }
                };
            }
        });
    }

    /// Creates a new [`Receiver`] handle that will receive values sent **after**
    /// this call to `subscribe`.
    pub fn subscribe(&self) -> Receiver<OwnedMessage> {
        self.sender.subscribe()
    }
}
