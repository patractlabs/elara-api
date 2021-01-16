use crate::error::ServiceError;
use crate::message::{MethodCall, Output, ResponseMessage, Value, Version};
use crate::session::{ArcSessions, Session, StorageKeys, SubscriptionSession};
use std::collections::HashSet;

#[allow(non_snake_case)]
pub(crate) async fn handle_state_subscribeStorage(
    sessions: ArcSessions,
    session: Session,
    request: MethodCall,
) -> Result<ResponseMessage, ServiceError> {
    let params: Vec<Vec<String>> = request.params.parse()?;
    let storage_keys = match params {
        arr if arr.is_empty() || arr.len() > 1 => {
            return Err(ServiceError::JsonrpcError(
                jsonrpc_core::Error::invalid_params("some params are invalid"),
            ));
        }
        arr if arr[0].is_empty() => StorageKeys::All,
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

   let mut sessions =  sessions.write().await;
    sessions.insert(session.clone(), storage_keys);
    let result = serde_json::to_string(&Output::from(
        // state_subscribeStorage's result is subscription id
        Ok(Value::String(sessions.subscription_id(&session).unwrap())),
        request.id,
        Some(Version::V2),
    ))?;

    Ok(ResponseMessage {
        id: session.client_id,
        chain: session.chain_name,
        result,
    })
}
