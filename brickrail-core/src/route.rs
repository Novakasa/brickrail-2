//! Route legs and their construction from a sensor trajectory.
//! See the "Route Legs" and "Leg Construction" sections of
//! `docs/design/train-simulation.md`.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::layout::block::BlockData;
use crate::layout::marker::MarkerData;
use crate::primitives::*;

/// A resolved route leg: one block-to-block traversal with its markers.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RouteLeg {
    /// The train's facing during this leg. Can change between legs when
    /// travel direction reverses.
    pub facing: Facing,
    /// The starting block as traversed in this leg.
    pub start_block: RouteLegBlock,
    /// The tracks between the start and target blocks.
    pub travel: Vec<DirectedTrackID>,
    /// The target block as traversed in this leg.
    pub target_block: RouteLegBlock,
    /// All markers along the sensor trajectory, in travel order.
    pub markers: Vec<RouteLegMarker>,
}

/// A block's participation in a route leg.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RouteLegBlock {
    pub block_id: BlockID,
    /// The direction the train traverses this block.
    pub direction: BlockDirection,
    /// The block's section as traversed in this leg (reversed relative to the
    /// stored section when travel is against it).
    pub section: Vec<DirectedTrackID>,
}

/// A marker within a resolved route leg, with its assigned role.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RouteLegMarker {
    /// The track this marker sits on.
    pub track: TrackID,
    /// The expected marker color for hardware validation.
    pub color: MarkerColor,
    /// The role of this marker in this leg, if any. `None` means the marker is
    /// only used for visual progress interpolation.
    pub role: Option<MarkerRole>,
    /// Position within the leg in marker-count units (0.0, 1.0, 2.0, ...).
    pub position: f32,
}

/// The role a marker plays within a specific route leg.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MarkerRole {
    /// First marker (requires 3+ markers) — the train is exiting the start block.
    Exiting,
    /// Second-to-last marker (requires 2+ markers) — the train is entering the target block.
    Entering,
    /// Last marker (always present) — the train has fully entered the target block.
    Entered,
}

impl RouteLeg {
    /// The logical block this leg starts from. Equals the train's position
    /// block while this leg is the front leg.
    pub fn start_logical_block_id(&self) -> LogicalBlockID {
        LogicalBlockID {
            block: self.start_block.block_id,
            direction: self.start_block.direction,
            facing: self.facing,
        }
    }

    /// The logical block this leg ends in. Becomes the train's position block
    /// when the leg completes.
    pub fn target_logical_block_id(&self) -> LogicalBlockID {
        LogicalBlockID {
            block: self.target_block.block_id,
            direction: self.target_block.direction,
            facing: self.facing,
        }
    }

    /// Build route legs from a path (sensor trajectory) through the logical
    /// graph. Splits the path at canonical enter markers and resolves each
    /// segment into a leg. Returns `None` if the path contains fewer than two
    /// enter markers.
    pub fn build_from_path(
        path: &[LogicalTrackID],
        block_data_map: &HashMap<BlockID, BlockData>,
        marker_data_map: &HashMap<TrackID, MarkerData>,
    ) -> Option<Vec<RouteLeg>> {
        let enter_marker_map = build_enter_marker_map(block_data_map);
        let segments = split_path_at_enter_markers(path, &enter_marker_map)?;

        segments
            .iter()
            .map(|slice| Self::build(slice, &enter_marker_map, block_data_map, marker_data_map))
            .collect()
    }

    /// Build a single leg from a path slice running enter marker to enter marker.
    fn build(
        sensor_trajectory: &[LogicalTrackID],
        enter_marker_map: &HashMap<LogicalTrackID, LogicalBlockID>,
        block_data_map: &HashMap<BlockID, BlockData>,
        marker_data_map: &HashMap<TrackID, MarkerData>,
    ) -> Option<RouteLeg> {
        let start = *enter_marker_map.get(sensor_trajectory.first()?)?;
        let target = *enter_marker_map.get(sensor_trajectory.last()?)?;

        let start_data = block_data_map.get(&start.block)?;
        let target_data = block_data_map.get(&target.block)?;

        let start_section = start_data.directed_section(start.direction);
        let target_section = target_data.directed_section(target.direction);

        // Travel tracks: everything in the slice that is in neither block's section.
        let start_tracks: HashSet<TrackID> = start_data.section.iter().map(|d| d.track).collect();
        let target_tracks: HashSet<TrackID> = target_data.section.iter().map(|d| d.track).collect();
        let travel: Vec<DirectedTrackID> = sensor_trajectory
            .iter()
            .filter(|lt| {
                !start_tracks.contains(&lt.track()) && !target_tracks.contains(&lt.track())
            })
            .map(|lt| lt.directed)
            .collect();

        let markers = collect_markers(sensor_trajectory, marker_data_map);

        Some(RouteLeg {
            facing: start.facing,
            start_block: RouteLegBlock {
                block_id: start.block,
                direction: start.direction,
                section: start_section,
            },
            travel,
            target_block: RouteLegBlock {
                block_id: target.block,
                direction: target.direction,
                section: target_section,
            },
            markers,
        })
    }
}

/// Map from logical track to logical block for every canonical enter marker.
pub fn build_enter_marker_map(
    block_data_map: &HashMap<BlockID, BlockData>,
) -> HashMap<LogicalTrackID, LogicalBlockID> {
    let mut map = HashMap::new();
    for (&block_id, data) in block_data_map {
        for direction in [BlockDirection::Aligned, BlockDirection::Against] {
            for facing in [Facing::Forward, Facing::Backward] {
                let logical_track = data.enter_logical_track(direction, facing);
                map.insert(
                    logical_track,
                    LogicalBlockID {
                        block: block_id,
                        direction,
                        facing,
                    },
                );
            }
        }
    }
    map
}

/// Split a path at enter markers into overlapping slices, each running from
/// one enter marker to the next, inclusive on both ends.
fn split_path_at_enter_markers<'a>(
    path: &'a [LogicalTrackID],
    enter_marker_map: &HashMap<LogicalTrackID, LogicalBlockID>,
) -> Option<Vec<&'a [LogicalTrackID]>> {
    let enter_indices: Vec<usize> = path
        .iter()
        .enumerate()
        .filter(|(_, lt)| enter_marker_map.contains_key(*lt))
        .map(|(i, _)| i)
        .collect();

    if enter_indices.len() < 2 {
        return None;
    }

    Some(
        enter_indices
            .windows(2)
            .map(|w| &path[w[0]..=w[1]])
            .collect(),
    )
}

/// Collect markers along the sensor trajectory and assign roles by position:
/// Entered is always the last marker; Entering the second-to-last when there
/// are at least two; Exiting the first when there are at least three; every
/// other marker has no role.
fn collect_markers(
    path_slice: &[LogicalTrackID],
    marker_data_map: &HashMap<TrackID, MarkerData>,
) -> Vec<RouteLegMarker> {
    let mut markers: Vec<(TrackID, MarkerColor, f32)> = Vec::new();
    for logical in path_slice {
        if let Some(data) = marker_data_map.get(&logical.track()) {
            let position = markers.len() as f32;
            markers.push((logical.track(), data.color, position));
        }
    }

    let len = markers.len();
    markers
        .into_iter()
        .enumerate()
        .map(|(idx, (track, color, position))| {
            let role = if idx + 1 == len {
                Some(MarkerRole::Entered)
            } else if len >= 2 && idx + 2 == len {
                Some(MarkerRole::Entering)
            } else if len >= 3 && idx == 0 {
                Some(MarkerRole::Exiting)
            } else {
                None
            };
            RouteLegMarker {
                track,
                color,
                role,
                position,
            }
        })
        .collect()
}
