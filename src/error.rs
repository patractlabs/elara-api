use actix_web_actors::ws::ProtocolError;
use anyhow::{Context, Result};
use rdkafka::error::KafkaError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServiceError {
    #[error(transparent)]
    KafkaError(#[from] KafkaError),

    #[error(transparent)]
    JsonrpcError(#[from] jsonrpc_core::Error),

    #[error(transparent)]
    WsError(#[from] ProtocolError),
}
