use bevy::prelude::*;

use super::SimulationSet;
use super::route::{MarkerRole, RouteLeg, TrainLegs};
use crate::layout::train::Train;
use crate::lifecycle::Registry;
use crate::primitives::*;
use bevy::ecs::relationship::RelationshipTarget;

/// The train's state within its front leg, derived from the marker index and
/// the roles of the leg's markers. Variants explicitly name which block
/// (start or target) they reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TrainLegState {
    /// Train has fully entered the target block.
    EnteredTarget,
    /// Train is exiting the start block.
    ExitingStart,
    /// Train is between blocks (after exiting start, before entering target).
    Outside,
    /// Train is entering the target block.
    EnteringTarget,
}

impl TrainLegState {
    /// The leg state when the train is at a marker with the given role.
    /// Exiting → ExitingStart (train is at the start block's exit marker).
    /// Entering → EnteringTarget (train is at the target block's enter marker).
    /// Entered → EnteredTarget (train has fully entered the target block).
    pub fn from_marker_role(role: Option<MarkerRole>) -> Self {
        match role {
            Some(MarkerRole::Exiting) => TrainLegState::ExitingStart,
            Some(MarkerRole::Entering) => TrainLegState::EnteringTarget,
            Some(MarkerRole::Entered) => TrainLegState::EnteredTarget,
            None => TrainLegState::Outside,
        }
    }
}

/// Simulation state: where a train is on the layout.
///
/// `block` is the logical block the train last fully entered. It is set by
/// `PlaceTrain` and updated to the completed leg's target on every
/// `CompleteLeg`. It is always the start block of the front leg in `TrainLegs`.
///
/// `marker_index` is the index of the last marker the train's sensor passed
/// within the front leg. It is zero whenever a leg becomes the front leg,
/// since a leg's first marker is the position block's enter marker, which the
/// train has already passed. With an empty queue it is zero.
///
/// A train with an empty leg queue is standing still in `block`; a train with
/// a non-empty queue is executing the front leg.
#[derive(Component, Clone, Debug, PartialEq, Eq)]
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

/// State event: place a train on the layout, standing in `block`.
/// The train's leg queue must be empty. Inserts (or overwrites) `TrainPosition`.
#[derive(Message, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PlaceTrain {
    pub train: TrainID,
    pub block: LogicalBlockID,
}

impl PlaceTrain {
    pub fn new(train: TrainID, block: LogicalBlockID) -> Self {
        Self { train, block }
    }
}

/// State event: the train passed the next marker of its front leg.
/// Increments `marker_index`.
#[derive(Message, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TrainMarkerHit {
    pub train: TrainID,
}

impl TrainMarkerHit {
    pub fn new(train: TrainID) -> Self {
        Self { train }
    }
}

/// State event: the front leg is complete. Removes it from the queue, sets
/// the position block to its target, and resets `marker_index` to zero.
/// Emitted by the logic layer once the train has passed the Entered marker.
#[derive(Message, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CompleteLeg {
    pub train: TrainID,
}

impl CompleteLeg {
    pub fn new(train: TrainID) -> Self {
        Self { train }
    }
}

/// Train position simulation state sub-plugin.
/// Registers the `PlaceTrain`, `TrainMarkerHit` and `CompleteLeg` state events.
pub struct TrainPositionStatePlugin;

impl Plugin for TrainPositionStatePlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PlaceTrain>();
        app.add_message::<TrainMarkerHit>();
        app.add_message::<CompleteLeg>();
        app.add_systems(
            Update,
            (
                handle_place_train.run_if(on_message::<PlaceTrain>),
                handle_train_marker_hit.run_if(on_message::<TrainMarkerHit>),
                handle_complete_leg.run_if(on_message::<CompleteLeg>),
            )
                .in_set(SimulationSet::StateMutation),
        );
    }
}

/// Mutation system for PlaceTrain. Sets the train's position to standing in the block.
fn handle_place_train(
    mut commands: Commands,
    mut messages: MessageReader<PlaceTrain>,
    train_registry: Res<Registry<Train>>,
    train_legs_query: Query<&TrainLegs>,
) {
    for msg in messages.read() {
        let train_entity = train_registry
            .get(&msg.train)
            .expect("PlaceTrain: train not found in registry");

        let queue_len = train_legs_query
            .get(train_entity)
            .map(|legs| legs.collection().len())
            .unwrap_or(0);
        assert_eq!(
            queue_len, 0,
            "PlaceTrain: train {:?} has {} queued legs; placement requires an empty queue",
            msg.train, queue_len
        );

        commands
            .entity(train_entity)
            .insert(TrainPosition::at(msg.block));
    }
}

/// Mutation system for TrainMarkerHit. Increments marker_index within the front leg.
fn handle_train_marker_hit(
    mut messages: MessageReader<TrainMarkerHit>,
    train_registry: Res<Registry<Train>>,
    mut train_position_query: Query<&mut TrainPosition>,
    train_legs_query: Query<&TrainLegs>,
    leg_query: Query<&RouteLeg>,
) {
    for msg in messages.read() {
        let train_entity = train_registry
            .get(&msg.train)
            .expect("TrainMarkerHit: train not found in registry");

        let mut position = train_position_query
            .get_mut(train_entity)
            .expect("TrainMarkerHit: train has no TrainPosition");

        let front_leg_entity = train_legs_query
            .get(train_entity)
            .ok()
            .and_then(|legs| legs.collection().first().copied())
            .expect("TrainMarkerHit: train has an empty leg queue");
        let front_leg = leg_query
            .get(front_leg_entity)
            .expect("TrainMarkerHit: front leg entity not found");

        assert!(
            position.marker_index + 1 < front_leg.markers.len(),
            "TrainMarkerHit: train {:?} is already past the last marker of its front leg",
            msg.train
        );
        position.marker_index += 1;
    }
}

/// Mutation system for CompleteLeg. Despawns the front leg, moves the position
/// block to its target, and resets marker_index.
fn handle_complete_leg(
    mut commands: Commands,
    mut messages: MessageReader<CompleteLeg>,
    train_registry: Res<Registry<Train>>,
    mut train_position_query: Query<&mut TrainPosition>,
    train_legs_query: Query<&TrainLegs>,
    leg_query: Query<&RouteLeg>,
) {
    for msg in messages.read() {
        let train_entity = train_registry
            .get(&msg.train)
            .expect("CompleteLeg: train not found in registry");

        let mut position = train_position_query
            .get_mut(train_entity)
            .expect("CompleteLeg: train has no TrainPosition");

        let front_leg_entity = train_legs_query
            .get(train_entity)
            .ok()
            .and_then(|legs| legs.collection().first().copied())
            .expect("CompleteLeg: train has an empty leg queue");
        let front_leg = leg_query
            .get(front_leg_entity)
            .expect("CompleteLeg: front leg entity not found");

        position.block = front_leg.target_logical_block_id();
        position.marker_index = 0;

        commands.entity(front_leg_entity).despawn();
    }
}
