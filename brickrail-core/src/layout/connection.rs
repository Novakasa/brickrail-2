use petgraph::graphmap::UnGraphMap;

use crate::primitives::{TrackConnectionID, TrackID};

/// Undirected structural graph of track connections. Every pathfinding
/// structure is derived from it.
#[derive(Debug, Default, Clone)]
pub struct ConnectionGraph {
    pub graph: UnGraphMap<TrackID, TrackConnectionID>,
}

impl ConnectionGraph {
    pub fn add(&mut self, conn: TrackConnectionID) {
        self.graph
            .add_edge(conn.track_a.track, conn.track_b.track, conn);
    }

    pub fn remove(&mut self, conn: &TrackConnectionID) {
        self.graph
            .remove_edge(conn.track_a.track, conn.track_b.track);
    }

    /// All connections from a given track.
    pub fn connections_from(&self, track: TrackID) -> Vec<TrackConnectionID> {
        self.graph.edges(track).map(|(_, _, conn)| *conn).collect()
    }

    /// Every connection in the graph.
    pub fn connections(&self) -> impl Iterator<Item = &TrackConnectionID> {
        self.graph.all_edges().map(|(_, _, conn)| conn)
    }
}
