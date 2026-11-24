use serde::{Deserialize, Serialize};

use crate::primitives::MarkerColor;

/// Layout data for a marker. A marker is identified by the track it sits on.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct MarkerData {
    pub color: MarkerColor,
}
