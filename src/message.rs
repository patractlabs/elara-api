use jsonrpc_core::response::{Failure, Success};
use jsonrpc_core::MethodCall;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Result, Value};

#[derive(Serialize, Deserialize, Debug)]
pub struct RequestMessage {
    pub id: String,
    pub chain: String,
    /// request is a subset of jsonrpc2 about request
    pub request: MethodCall,
}

pub type ResponseSuccessMessage = Success;
pub type ResponseFailureMessage = Failure;

/// SubscribedMessage is a subset of jsonrpc2 about subscription data
#[derive(Serialize, Deserialize, Debug)]
pub struct SubscribedMessage {
    pub id: String,
    pub chain: String,
    pub params: SubscribedParams,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SubscribedParams {
    pub subscription: String,
    pub result: Vec<Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message() -> Result<()> {
        // Some JSON input data as a &str. Maybe this comes from the user.
        let request_data = r#"
       {
            "id": "0x0123456789ABCDEF",
            "chain": "test-net",
            "request": {
                "id": 233,
                "jsonrpc": "2.0",
                "method": "state_subscribeStorage",
                "params": []
            }
        }"#;

        let v: RequestMessage = serde_json::from_str(request_data)?;
        println!("{:#?}", v);

        let result_data = r#"
{
    "id": "b6c6d0aa16b0f5eb65e6fd87c6ffbba2",
    "chain": "polkadot",
    "result": [{
        "block": "0x04b67ec2b6ff34ebd58ed95fe9aad1068f805d2519ca8a24b986994b6764f410",
        "number": 10086,
        "is_full": true,
        "changes": [
            ["0x2aeddc77fe58c98d50bd37f1b90840f9cd7f37317cd20b61e9bd46fab870471456c62bce26605ee05c3c4c795e554a782e59ef5043ca9772f32dfb1ad7de832878d662194193955e",
                null
            ],
            ["0x2aeddc77fe58c98d50bd37f1b90840f943a953ac082e08b6527ce262dbd4abf2e7731c5a045ae2174d185feff2d91e9a5c3c4c795e554a782e59ef5043ca9772f32dfb1ad7de832878d662194193955e",
                "0x3a875e45c13575f66eadb2d60608df9068a90e46ed33723098021e8cedd67d3a09f09f90ad20584949 "
            ]
        ]
    }]
}

        "#;

        let v: ResponseMessage = serde_json::from_str(result_data)?;
        println!("{:#?}", v);

        Ok(())
    }
}
