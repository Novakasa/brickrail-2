use serde::{Deserialize, Serialize};

/// Layout data for a train definition.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TrainData {
    pub name: String,
}
