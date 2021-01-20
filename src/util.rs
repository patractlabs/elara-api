use crate::message::{Error, MethodCall, Success, Value, Version};
use crate::session::{Session, StorageKeys, StorageSessions};
use std::collections::HashSet;

// TODO: refine these as a trait

#[allow(non_snake_case)]
pub(crate) fn handle_state_unsubscribeStorage(
    sessions: &mut StorageSessions,
    _session: Session,
    request: MethodCall,
) -> Result<Success, Error> {
    let params = request.params.parse::<(String,)>()?;
    let subscribed = sessions.remove(&params.0.into()).is_some();
    Ok(Success {
        jsonrpc: Some(Version::V2),
        result: Value::Bool(subscribed),
        id: request.id,
    })
}

#[allow(non_snake_case)]
pub(crate) fn handle_state_subscribeStorage(
    sessions: &mut StorageSessions,
    session: Session,
    request: MethodCall,
) -> Result<Success, Error> {
    let params: Vec<Vec<String>> = request.params.parse()?;
    // TODO: make sure the api semantics
    let storage_keys = match params {
        arr if arr.len() > 1 => {
            return Err(Error::invalid_params("more than one param"));
        }
        arr if arr.is_empty() || arr[0].is_empty() => StorageKeys::All,
        arrs => {
            let arr = &arrs[0];
            let len = arr.len();
            let keys = arr
                .iter()
                .map(|v| v.to_string())
                .collect::<HashSet<String>>();

            // TODO: try to keep same behavior with substrate
            if len != keys.len() {
                return Err(Error::invalid_params("some keys are invalid"));
            }
            StorageKeys::Some(keys)
        }
    };

    let id = sessions.new_subscription_id();
    sessions.insert(id.clone(), (session.clone(), storage_keys));
    Ok(Success {
        jsonrpc: Some(Version::V2),
        result: Value::from(id),
        id: request.id,
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

        let success = handle_state_subscribeStorage(&mut sessions, session, request).unwrap();

        // unsubscribe
        let session = Session {
            chain_name: "test-net".to_string(),
            client_id: "0x2".to_string(),
        };
        let request = MethodCall {
            jsonrpc: Some(Version::V2),
            method: "state_unsubscribeStorage".to_string(),
            params: Params::Array(vec![Value::String(
                success.result.as_str().unwrap().to_string(),
            )]),
            id: Id::Num(2),
        };

        let success =
            handle_state_unsubscribeStorage(&mut sessions, session.clone(), request.clone())
                .unwrap();
        assert_eq!(
            success,
            Success {
                jsonrpc: Some(Version::V2),
                result: Value::Bool(true),
                id: Id::Num(2),
            }
        );

        // unsubscribe again
        let success = handle_state_unsubscribeStorage(&mut sessions, session, request).unwrap();
        assert_eq!(
            success,
            Success {
                jsonrpc: Some(Version::V2),
                result: Value::Bool(false),
                id: Id::Num(2),
            }
        );
    }
}
