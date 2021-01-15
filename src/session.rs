use crate::message::RequestMessage;
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

// TODO: choose better struct
// TODO: refine
// map (chain_name ++ client_id) -> (keys set)

/// SubscriptionSession maintains the ws sessions for different subscriptions for one connection
#[derive(Debug, Default)]
pub struct SubscriptionSession(pub HashMap<Session, StorageKeys<HashSet<String>>>);

pub type PeerSubscriptionSession = Arc<RwLock<HashMap<SocketAddr, SubscriptionSession>>>;

pub type SubscriptionAllSession = HashSet<Session>;

#[derive(Clone, Debug)]
pub enum StorageKeys<T> {
    All,
    Some(T),
}

/// RouteKey as a subscription session
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Session {
    pub chain_name: String,
    pub client_id: String,
}

impl From<&RequestMessage> for Session {
    fn from(msg: &RequestMessage) -> Self {
        Self {
            chain_name: msg.chain.clone(),
            client_id: msg.id.clone(),
        }
    }
}
