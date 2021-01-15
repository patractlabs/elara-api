use rdkafka::error::KafkaError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error(transparent)]
    KafkaError(#[from] KafkaError),

    #[error(transparent)]
    JsonrpcError(#[from] jsonrpc_core::Error),

    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, ServiceError>;
