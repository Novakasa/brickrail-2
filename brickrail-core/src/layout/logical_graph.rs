use petgraph::graphmap::DiGraphMap;

use super::block::BlockData;
use crate::primitives::{Facing, LogicalTrackID, TrackConnectionID};

/// Directed graph of logical tracks (directed track + facing) on which
/// pathfinding and leg construction operate.
///
/// Two kinds of edges:
/// - **Connection edges** preserve facing: four per physical connection.
/// - **Flip edges** at a block's endpoint tracks join a logical track to its
///   reversed variant: two per block. Reversal happens only inside blocks.
pub type LogicalGraph = DiGraphMap<LogicalTrackID, ()>;

/// Builds the logical graph from the connections and the blocks.
pub fn build_logical_graph<'a>(
    connections: impl IntoIterator<Item = &'a TrackConnectionID>,
    blocks: impl IntoIterator<Item = &'a BlockData>,
) -> LogicalGraph {
    let mut graph = DiGraphMap::new();

    // A connection has track_a and track_b pointing toward each other.
    // Traveling A→B: the train is on track_a's direction; after crossing it is
    // on track_b.opposite() (heading away from A). Same facing on both sides.
    for conn in connections {
        for facing in [Facing::Forward, Facing::Backward] {
            let from = LogicalTrackID::new(conn.track_a, facing);
            let to = LogicalTrackID::new(conn.track_b.opposite(), facing);
            graph.add_edge(from, to, ());

            let from = LogicalTrackID::new(conn.track_b, facing);
            let to = LogicalTrackID::new(conn.track_a.opposite(), facing);
            graph.add_edge(from, to, ());
        }
    }

    // Flip edges at each block's two endpoint tracks. For a section [t0, ..., tn]:
    //   at tn: (tn, Forward) ↔ its reversed variant
    //     — connects (Aligned, Forward) enter to (Against, Backward) enter
    //   at t0: (t0.opposite(), Forward) ↔ its reversed variant
    //     — connects (Against, Forward) enter to (Aligned, Backward) enter
    for data in blocks {
        let (Some(t0), Some(tn)) = (data.section.first(), data.section.last()) else {
            continue;
        };

        let logical_tn = LogicalTrackID::new(*tn, Facing::Forward);
        graph.add_edge(logical_tn, logical_tn.reversed(), ());
        graph.add_edge(logical_tn.reversed(), logical_tn, ());

        let logical_t0 = LogicalTrackID::new(t0.opposite(), Facing::Forward);
        graph.add_edge(logical_t0, logical_t0.reversed(), ());
        graph.add_edge(logical_t0.reversed(), logical_t0, ());
    }

    graph
}
