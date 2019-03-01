use serde::{Deserialize, Serialize};

use sc2_proto::sc2api::Request;

/// Incoming request access control
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
pub struct RequestLimits {
    #[serde(default)]
    pub disable_cheats: bool,
}
impl RequestLimits {
    pub fn is_request_allowed(&self, _req: &Request) -> bool {
        // TODO: Access control
        true
    }
}
