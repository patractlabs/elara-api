use crate::rpc_api::SubscribedResult;
use serde::{Deserialize, Serialize};

use crate::error::ServiceError;
pub use jsonrpc_core::{Error, Failure, Id, MethodCall, Output, Params, Success, Value, Version};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RequestMessage {
    pub id: String,
    pub chain: String,
    /// A jsonrpc string about request
    pub request: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct ResponseMessage {
    pub id: String,
    pub chain: String,
    /// A jsonrpc string about result
    pub result: String,
}

/// When request is illegal, the message will be returned.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResponseErrorMessage {
    pub error: String,
}

impl From<ServiceError> for ResponseErrorMessage {
    fn from(err: ServiceError) -> Self {
        Self {
            error: err.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubscribedMessage {
    pub id: String,
    pub chain: String,
    /// A jsonrpc string about subscription
    pub data: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubscribedData {
    pub jsonrpc: Option<Version>,
    pub method: String,
    pub params: SubscribedParams,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubscribedParams {
    pub subscription: SubscriptionId,
    pub result: SubscribedResult,
}

// Note: Altered from jsonrpc_pubsub::SubscriptionId

/// Unique subscription id.
///
/// NOTE Assigning same id to different requests will cause the previous request to be unsubscribed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(untagged)]
pub enum SubscriptionId {
    /// A numerical ID, represented by a `u64`.
    Number(u64),
    /// A non-numerical ID, for example a hash.
    String(String),
}

impl SubscriptionId {
    /// Parses `core::Value` into unique subscription id.
    pub fn parse_value(val: &Value) -> Option<SubscriptionId> {
        match *val {
            Value::String(ref val) => Some(SubscriptionId::String(val.clone())),
            Value::Number(ref val) => val.as_u64().map(SubscriptionId::Number),
            _ => None,
        }
    }
}

impl From<String> for SubscriptionId {
    fn from(other: String) -> Self {
        SubscriptionId::String(other)
    }
}

impl From<SubscriptionId> for Value {
    fn from(sub: SubscriptionId) -> Self {
        match sub {
            SubscriptionId::Number(val) => Value::Number(val.into()),
            SubscriptionId::String(val) => Value::String(val),
        }
    }
}

macro_rules! impl_from_num {
    ($num:ty) => {
        impl From<$num> for SubscriptionId {
            fn from(other: $num) -> Self {
                SubscriptionId::Number(other.into())
            }
        }
    };
}

impl_from_num!(u8);
impl_from_num!(u16);
impl_from_num!(u32);
impl_from_num!(u64);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Result;

    #[test]
    fn test_request_message() -> Result<()> {
        // Some JSON input data as a &str. Maybe this comes from the user.
        let request_data = r#"
{
    "id": "b6c6d0aa16b0f5eb65e6fd87c6ffbba2",
    "chain": "polkadot",
    "request": "{\n\"id\": 141,\n\"jsonrpc\": \"2.0\",\n\"method\": \"state_subscribeStorage\",\n\"params\": [ [\"0x2aeddc77fe58c98d50bd37f1b90840f9cd7f37317cd20b61e9bd46fab87047149c21b6ab44c00eb3127a30e486492921e58f2564b36ab1ca21ff630672f0e76920edd601f8f2b89a\"]]}"
}
"#;

        let v: RequestMessage = serde_json::from_str(request_data)?;
        let _v: MethodCall = serde_json::from_str(&*v.request)?;

        Ok(())
    }

    #[test]
    fn test_response_message() -> Result<()> {
        let data = r#"
{
    "id": "b6c6d0aa16b0f5eb65e6fd87c6ffbba2",
    "chain": "polkadot",
    "result": "{\n\"jsonrpc\": \"2.0\",\n\"result\": \"0x91b171bb158e2d3848fa23a9f1c25182fb8e20313b2c1eb49219da7a70ce90c3\",\"id\": 1}"
}
"#;

        let v: ResponseMessage = serde_json::from_str(data)?;
        let _v: Success = serde_json::from_str(&*v.result)?;

        Ok(())
    }

    #[test]
    fn test_subscribed_message() -> Result<()> {
        let msg = r#"
{
    "id": "b6c6d0aa16b0f5eb65e6fd87c6ffbba2",
    "chain": "polkadot",
    "data": "{\"jsonrpc\": \"2.0\",\n\"method\":\"state_storage\", \n\"params\": {\n\"subscription\": \"ffMpMJgyQt3rmHx8\",\n\t\t\"result\": {\n\t\t  \"block\": \"0x04b67ec2b6ff34ebd58ed95fe9aad1068f805d2519ca8a24b986994b6764f410\",\n\t\t  \"changes\": [\n    [\"0x2aeddc77fe58c98d50bd37f1b90840f9cd7f37317cd20b61e9bd46fab870471456c62bce26605ee05c3c4c795e554a782e59ef5043ca9772f32dfb1ad7de832878d662194193955e\",              null ],[\"0x2aeddc77fe58c98d50bd37f1b90840f943a953ac082e08b6527ce262dbd4abf2e7731c5a045ae2174d185feff2d91e9a5c3c4c795e554a782e59ef5043ca9772f32dfb1ad7de832878d662194193955e\", \"0x3a875e45c13575f66eadb2d60608df9068a90e46ed33723098021e8cedd67d3a09f09f90ad20584949\"]]}}}"
}
"#;

        let v: SubscribedMessage = serde_json::from_str(msg)?;
        dbg!(&v.data);
        let _v: SubscribedData = serde_json::from_str(&*v.data)?;

        Ok(())
    }
}
