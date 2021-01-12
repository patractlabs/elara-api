use rdkafka::config::{ClientConfig, RDKafkaLogLevel};
use rdkafka::consumer::stream_consumer::StreamConsumer;
use rdkafka::consumer::{CommitMode, Consumer, ConsumerContext, Rebalance};
use rdkafka::error::KafkaResult;
use rdkafka::topic_partition_list::TopicPartitionList;
use rdkafka::ClientContext;
use std::collections::{HashMap, HashSet};

struct KVContext;

impl ClientContext for KVContext {}

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

pub type KvConsumer = StreamConsumer<KVContext>;

impl KvConsumer {
    fn new<T>(config: T) -> Self
    where T: Iterator<Item = (impl Into<String>, impl Into<String>)>
    {
        let context = KVContext {};

        // TODO: arrange config
        let cfg = ClientConfig::new()
            .set("group.id", group_id)
            .set("bootstrap.servers", brokers)
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("enable.auto.commit", "true")
            .set_log_level(RDKafkaLogLevel::Debug);

        for (k, v) in config {
            cfg.set(k.into().as_str(), v.into().as_str());
        }

        let consumer: KvConsumer = cfg
            .create_with_context(context)
            .expect("Consumer creation failed");

        consumer
    }
}
