use jsonrpc_core::response::{Failure, Success};
pub use jsonrpc_core::{Error, MethodCall, Output, Params, Version};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Result, Value};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RequestMessage {
    pub id: String,
    pub chain: String,
    /// A subset of jsonrpc2 about request
    pub request: MethodCall,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ResponseMessage {
    pub id: String,
    pub chain: String,
    // TODO: check the format
    /// A subset of jsonrpc2 about response
    pub result: Output,
}

pub type ResponseSuccessMessage = Success;
pub type ResponseFailureMessage = Failure;

/// SubscribedMessage is a subset of jsonrpc2 about subscription data
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubscribedMessage {
    pub id: String,
    pub chain: String,
    /// A subset of jsonrpc2 about subscription
    pub data: SubscribedData,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubscribedData {
    pub jsonrpc: Option<Version>,
    pub params: SubscribedParams,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SubscribedParams {
    pub subscription: String,
    pub result: Vec<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_message() -> Result<()> {
        // Some JSON input data as a &str. Maybe this comes from the user.
        let request_data = r#"
{
    "id": "b6c6d0aa16b0f5eb65e6fd87c6ffbba2",
    "chain": "polkadot",
    "request": {
        "id": 141,
        "jsonrpc": "2.0",
        "method": "state_subscribeStorage",
        "params": [ ["0x2aeddc77fe58c98d50bd37f1b90840f9cd7f37317cd20b61e9bd46fab87047149c21b6ab44c00eb3127a30e486492921e58f2564b36ab1ca21ff630672f0e76920edd601f8f2b89a"]]
    }
}
"#;

        let v: RequestMessage = serde_json::from_str(request_data)?;
        Ok(())
    }

    #[test]
    fn test_response_message() -> Result<()> {
        let data = r#"
    {
      "id": "b6c6d0aa16b0f5eb65e6fd87c6ffbba2",
      "chain": "polkadot",
      "result": {
        "jsonrpc": "2.0",
        "result": "0x91b171bb158e2d3848fa23a9f1c25182fb8e20313b2c1eb49219da7a70ce90c3",
        "id": 1
      }
    }
"#;

        let v: ResponseMessage = serde_json::from_str(data)?;

        Ok(())
    }

    #[test]
    fn test_subscribed_message() -> Result<()> {
        let data = r#"
{
  "id": "b6c6d0aa16b0f5eb65e6fd87c6ffbba2",
  "chain": "polkadot",
  "data": {
    "jsonrpc": "2.0",
    "params": {
      "subscription": "ffMpMJgyQt3rmHx8",
      "result": [{
        "block": "0x04b67ec2b6ff34ebd58ed95fe9aad1068f805d2519ca8a24b986994b6764f410",
        "number": 10086,
        "is_full": true,
        "changes": [
          ["0x2aeddc77fe58c98d50bd37f1b90840f9cd7f37317cd20b61e9bd46fab870471456c62bce26605ee05c3c4c795e554a782e59ef5043ca9772f32dfb1ad7de832878d662194193955e",
            null
          ],
          ["0x2aeddc77fe58c98d50bd37f1b90840f943a953ac082e08b6527ce262dbd4abf2e7731c5a045ae2174d185feff2d91e9a5c3c4c795e554a782e59ef5043ca9772f32dfb1ad7de832878d662194193955e",
            "0x3a875e45c13575f66eadb2d60608df9068a90e46ed33723098021e8cedd67d3a09f09f90ad20584949"
          ]
        ]
      }]
    }
  }
}
"#;

        let v: SubscribedMessage = serde_json::from_str(data)?;

        Ok(())
    }
}
