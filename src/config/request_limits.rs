use serde::{Deserialize, Serialize};

use sc2_proto::sc2api::Request;

/// Incoming request access control
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq)]
pub struct RequestLimits {
    /// Cheats (all debug commands except drawing)
    #[serde(default)]
    pub disable_cheats: bool,
}
impl RequestLimits {
    /// Checks if the limits here allow a particular request
    pub fn is_request_allowed(&self, req: &Request) -> bool {
        if self.disable_cheats && req.has_debug() {
            let req_debugs = req.get_debug();
            if req_debugs.get_debug().iter().any(|r| !r.has_draw()) {
                return false;
            }
        }

        true
    }
}
