//! SC2 data and types

use serde::{Deserialize, Serialize};

#[allow(missing_docs)]
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum Race {
    Protoss,
    Terran,
    Zerg,
    Random,
}
