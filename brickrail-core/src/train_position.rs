//! Where a train is: its position block and progress along its front leg.
//! See "Train Position" in `docs/design/train-simulation.md`.

use serde::{Deserialize, Serialize};

use crate::primitives::LogicalBlockID;
use crate::route::{MarkerRole, RouteLeg};

/// The train's state within its front leg, derived from the marker index and
/// the roles of the leg's markers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrainLegState {
    /// Train has fully entered the target block.
    EnteredTarget,
    /// Train is exiting the start block.
    ExitingStart,
    /// Train is between blocks.
    Outside,
    /// Train is entering the target block.
    EnteringTarget,
}

impl TrainLegState {
    /// The leg state when the train is at a marker with the given role.
    pub fn from_marker_role(role: Option<MarkerRole>) -> Self {
        match role {
            Some(MarkerRole::Exiting) => TrainLegState::ExitingStart,
            Some(MarkerRole::Entering) => TrainLegState::EnteringTarget,
            Some(MarkerRole::Entered) => TrainLegState::EnteredTarget,
            None => TrainLegState::Outside,
        }
    }
}

/// Where a train is on the layout.
///
/// `block` is the logical block the train last fully entered: set by
/// placement, then updated to each completed leg's target. It is always the
/// start block of the front leg.
///
/// `marker_index` is the index of the last marker the train's sensor passed
/// within the front leg; zero whenever a leg becomes the front leg, and zero
/// with an empty queue.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrainPosition {
    pub block: LogicalBlockID,
    pub marker_index: usize,
}

impl TrainPosition {
    /// A train standing still in `block`.
    pub fn at(block: LogicalBlockID) -> Self {
        Self {
            block,
            marker_index: 0,
        }
    }

    /// The train's state within `front_leg`, derived from the marker index.
    pub fn leg_state(&self, front_leg: &RouteLeg) -> TrainLegState {
        TrainLegState::from_marker_role(front_leg.markers[self.marker_index].role)
    }

    /// Whether the train has passed `front_leg`'s Entered marker.
    pub fn has_completed(&self, front_leg: &RouteLeg) -> bool {
        self.marker_index + 1 == front_leg.markers.len()
    }
}
