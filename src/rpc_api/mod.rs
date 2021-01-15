pub mod state;

use serde::{Deserialize, Serialize};
use state::*;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(untagged)]
pub enum SubscribedResult {
    StateStorageResult(StateStorageResult),
}
