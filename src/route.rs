use crate::message::RequestMessage;
use std::collections::{HashMap, HashSet};

// TODO: choose better struct
// TODO: refine
// map (chain_name ++ client_id) -> (keys set)
#[derive(Debug, Default)]
pub struct SubscriptionRoute(pub HashMap<RouteKey, StorageKeys<HashSet<String>>>);

#[derive(Clone, Debug)]
pub enum StorageKeys<T> {
    All,
    Some(T),
}

/// RouteKey as a subscription session
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RouteKey {
    pub chain_name: String,
    pub client_id: String,
}

impl From<&RequestMessage> for RouteKey {
    fn from(msg: &RequestMessage) -> Self {
        Self {
            chain_name: msg.chain.clone(),
            client_id: msg.id.clone(),
        }
    }
}
