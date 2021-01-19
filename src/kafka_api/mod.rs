use serde::{Deserialize, Serialize};

pub type KafkaStoragePayload = Vec<KafkaStoragePayloadItem>;

// TODO: make sure kafka api from archive side
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct KafkaStoragePayloadItem {
    pub id: u64,
    pub block_num: u64,
    pub hash: String,
    pub is_full: bool,
    pub key: String,
    pub storage: Option<String>,
}
