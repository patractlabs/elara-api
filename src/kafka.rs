use rdkafka::ClientContext;
use rdkafka::config::{ClientConfig, RDKafkaLogLevel};

struct KVContext;

impl ClientContext for KVContext {}

