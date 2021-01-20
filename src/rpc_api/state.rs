use crate::kafka_api::KafkaStoragePayload;
use crate::rpc_api::*;
use serde::{Deserialize, Serialize};

/// storage data as Subscribed data in `result`
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StateStorageResult {
    pub block: String,
    pub changes: Vec<(String, Option<String>)>,
}

impl From<StateStorageResult> for SubscribedResult {
    fn from(res: StateStorageResult) -> Self {
        Self::StateStorageResult(res)
    }
}

impl From<&KafkaStoragePayload> for StateStorageResult {
    fn from(payload: &KafkaStoragePayload) -> Self {
        Self {
            // assume payload at least have one
            block: payload[0].hash.clone(),
            changes: payload
                .iter()
                .map(|item| (item.key.clone(), item.storage.clone()))
                .collect(),
        }
    }
}
