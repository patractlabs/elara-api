//! Some api handle logics
use crate::error::ServiceError;
use crate::message::{MethodCall, Output, ResponseMessage, Value, Version};
use crate::session::{Session, StorageKeys, StorageSessions};
use std::collections::HashSet;


#[allow(non_snake_case)]
pub(crate) fn handle_state_unsubscribeStorage(
    sessions: &mut StorageSessions,
    session: Session,
    request: MethodCall,
) -> Result<ResponseMessage, ServiceError> {
    let params: (String,) = request.params.parse()?;
    // remove the session
    let subscribed = if let Some(_) = sessions.remove(&params.0.into()) {
        true
    } else {
        // we don't find the subscription id
        false
    };

    let result = serde_json::to_string(&Output::from(
        // state_subscribeStorage's result is subscription id
        Ok(Value::Bool(subscribed)),
        request.id,
        Some(Version::V2),
    ))?;

    Ok(ResponseMessage {
        id: session.client_id,
        chain: session.chain_name,
        result,
    })
}

#[allow(non_snake_case)]
pub(crate) fn handle_state_subscribeStorage(
    sessions: &mut StorageSessions,
    session: Session,
    request: MethodCall,
) -> Result<ResponseMessage, ServiceError> {
    let params: Vec<Vec<String>> = request.params.parse()?;
    // TODO: make sure the api semantics
    let storage_keys = match params {
        arr if arr.len() > 1 => {
            return Err(ServiceError::JsonrpcError(
                jsonrpc_core::Error::invalid_params("some params are invalid"),
            ));
        }
        arr if arr.is_empty() || arr[0].is_empty() => StorageKeys::All,
        arrs => {
            let arr = &arrs[0];
            let len = arr.len();
            let keys = arr
                .iter()
                .map(|v| v.to_string())
                .collect::<HashSet<String>>();

            // TODO: keep same behavior with substrate
            if len != keys.len() {
                return Err(ServiceError::JsonrpcError(
                    jsonrpc_core::Error::invalid_params("some params are invalid"),
                ));
            }
            StorageKeys::Some(keys)
        }
    };

    let id = sessions.new_subscription_id();
    sessions.insert(id.clone(), (session.clone(), storage_keys));
    let result = serde_json::to_string(&Output::from(
        // state_subscribeStorage's result is subscription id
        Ok(id.into()),
        request.id,
        Some(Version::V2),
    ))?;

    Ok(ResponseMessage {
        id: session.client_id,
        chain: session.chain_name,
        result,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{Params, Success};
    use crate::session::Sessions;
    use jsonrpc_core::Id;

    #[allow(non_snake_case)]
    #[tokio::test]
    async fn test_state_subscribeStorage() {
        let mut sessions = Sessions::default();

        // subscribe
        let session = Session {
            chain_name: "test-net".to_string(),
            client_id: "0x1".to_string(),
        };

        let request: MethodCall = serde_json::from_str(
            r##"
{
  "jsonrpc": "2.0",
  "method": "state_subscribeStorage",
  "params": [],
  "id": 1
}
        "##,
        )
        .unwrap();

        let resp = handle_state_subscribeStorage(&mut sessions, session.clone(), request).unwrap();

        let result: Success = serde_json::from_str(&*resp.result).unwrap();

        // unsubscribe
        let session = Session {
            chain_name: "test-net".to_string(),
            client_id: "0x2".to_string(),
        };
        let request = MethodCall {
            jsonrpc: Some(Version::V2),
            method: "state_unsubscribeStorage".to_string(),
            params: Params::Array(vec![Value::String(
                result.result.as_str().unwrap().to_string(),
            )]),
            id: Id::Num(2),
        };

        let resp = handle_state_unsubscribeStorage(&mut sessions, session.clone(), request.clone())
            .unwrap();
        assert_eq!(
            resp,
            ResponseMessage {
                id: "0x2".to_string(),
                chain: "test-net".to_string(),
                result: serde_json::to_string(&Success {
                    jsonrpc: Some(Version::V2),
                    result: Value::Bool(true),
                    id: Id::Num(2),
                })
                .unwrap(),
            }
        );

        // unsubscribe again
        let resp = handle_state_unsubscribeStorage(&mut sessions, session, request).unwrap();
        assert_eq!(
            resp,
            ResponseMessage {
                id: "0x2".to_string(),
                chain: "test-net".to_string(),
                result: serde_json::to_string(&Success {
                    jsonrpc: Some(Version::V2),
                    result: Value::Bool(false),
                    id: Id::Num(2),
                })
                .unwrap(),
            }
        );
    }
}
