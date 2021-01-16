use crate::message::RequestMessage;
use std::collections::hash_map::Iter;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

// TODO: support other session type for other subscription type
// map (chain_name ++ client_id) -> (keys set)

// TODO: remove
/// SubscriptionSession maintains the ws sessions for different subscriptions for one connection
#[derive(Debug, Default)]
pub struct SubscriptionSession(pub HashMap<Session, StorageKeys<HashSet<String>>>);
// TODO: remove
pub type ArcSubscriptionSession = Arc<RwLock<SubscriptionSession>>;

/// Sessions maintains the ws sessions for different subscriptions for one connection
#[derive(Default, Debug, Clone)]
pub struct Sessions(HashMap<Session, (SubscriptionId, StorageKeys<HashSet<String>>)>);

/// SubscriptionId is generated according to Session uniquely.
pub type SubscriptionId = String;

pub type ArcSessions = Arc<RwLock<Sessions>>;

impl Sessions {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn insert(&mut self, session: Session, keys: StorageKeys<HashSet<String>>) {
        // TODO: make sure the map for id
        let id = session.client_id.clone() + &*session.chain_name;
        self.0.insert(session, (id, keys));
    }

    pub fn remove(&mut self, session: &Session) {
        self.0.remove(session);
    }

    pub fn subscription_id(&self, session: &Session) -> Option<SubscriptionId> {
        self.0.get(session).map(|(id, _)| id.to_string())
    }

    pub fn iter(&self) -> Iter<'_, Session, (SubscriptionId, StorageKeys<HashSet<String>>)> {
        self.0.iter()
    }
}

/// All represent for all storage keys.
/// Some contains some keys.
#[derive(Clone, Debug)]
pub enum StorageKeys<T> {
    All,
    Some(T),
}

/// Session as a subscription session
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
