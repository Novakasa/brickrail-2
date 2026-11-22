use bevy::ecs::relationship::RelationshipTarget;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use brickrail_common::layout::LayoutAppPlugin;
use brickrail_common::layout::block::{Block, BlockData};
use brickrail_common::layout::connection::Connection;
use brickrail_common::layout::logical_graph::LogicalGraph;
use brickrail_common::layout::marker::{Marker, MarkerData};
use brickrail_common::layout::track::Track;
use brickrail_common::layout::train::Train;
use brickrail_common::lifecycle::*;
use brickrail_common::primitives::*;
use brickrail_common::simulation::SimulationLogicPlugin;
use brickrail_common::simulation::route::{AppendLegs, RouteLeg, TrainLegs};
use brickrail_common::simulation::train_position::{PlaceTrain, TrainPosition};
use brickrail_common::simulation::virtual_driver::{VirtualDriver, VirtualDriverPlugin};
use petgraph::algo::astar;

fn make_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(LayoutAppPlugin);
    app.add_plugins(SimulationLogicPlugin);
    app.add_plugins(VirtualDriverPlugin);
    app
}

/// Spawn a two-block layout: [A: t0-t1] -- t2 -- [B: t3-t4]
fn spawn_two_block_layout(app: &mut App) {
    let t0 = TrackID::new(CellID::new(0, 0, 0), Orientation::EW);
    let t1 = TrackID::new(CellID::new(1, 0, 0), Orientation::EW);
    let t2 = TrackID::new(CellID::new(2, 0, 0), Orientation::EW);
    let t3 = TrackID::new(CellID::new(3, 0, 0), Orientation::EW);
    let t4 = TrackID::new(CellID::new(4, 0, 0), Orientation::EW);

    let world = app.world_mut();
    for t in [t0, t1, t2, t3, t4] {
        world.spawn_element::<Track>(t, Default::default());
    }
    for (a, b) in [(t0, t1), (t1, t2), (t2, t3), (t3, t4)] {
        world.spawn_element::<Connection>(a.get_connection_to(b).unwrap(), Default::default());
    }
    for t in [t0, t1, t3, t4] {
        world.spawn_element::<Marker>(t, Default::default());
    }
    world.spawn_element::<Block>(
        BlockID::new(t0, t1),
        BlockData {
            name: Some("A".to_string()),
            section: vec![
                t0.get_directed_to(Cardinal::E).unwrap(),
                t1.get_directed_to(Cardinal::E).unwrap(),
            ],
            ..Default::default()
        },
    );
    world.spawn_element::<Block>(
        BlockID::new(t3, t4),
        BlockData {
            name: Some("B".to_string()),
            section: vec![
                t3.get_directed_to(Cardinal::E).unwrap(),
                t4.get_directed_to(Cardinal::E).unwrap(),
            ],
            ..Default::default()
        },
    );
    world.spawn_element::<Train>(TrainID(0), Default::default());

    app.update();
}

fn extract_block_data_map(app: &App) -> HashMap<BlockID, BlockData> {
    let block_registry = app.world().resource::<Registry<Block>>();
    let mut map = HashMap::new();
    for (id, &entity) in block_registry.iter() {
        let data = app.world().get::<ElementData<Block>>(entity).unwrap();
        map.insert(*id, data.0.clone());
    }
    map
}

fn extract_marker_data_map(app: &App) -> HashMap<TrackID, MarkerData> {
    let marker_registry = app.world().resource::<Registry<Marker>>();
    let mut map = HashMap::new();
    for (id, &entity) in marker_registry.iter() {
        let data = app.world().get::<ElementData<Marker>>(entity).unwrap();
        map.insert(*id, data.0.clone());
    }
    map
}

fn build_route(
    start: LogicalBlockID,
    target: LogicalBlockID,
    logical_graph: &LogicalGraph,
    block_data_map: &HashMap<BlockID, BlockData>,
    marker_data_map: &HashMap<TrackID, MarkerData>,
) -> Option<Vec<RouteLeg>> {
    let start_data = block_data_map.get(&start.block)?;
    let target_data = block_data_map.get(&target.block)?;
    let start_track = start_data.enter_logical_track(start.direction, start.facing);
    let target_track = target_data.enter_logical_track(target.direction, target.facing);

    let (_cost, path) = astar(
        &logical_graph.graph,
        start_track,
        |n| n == target_track,
        |_| 1u32,
        |_| 0u32,
    )?;

    RouteLeg::build_from_path(&path, block_data_map, marker_data_map)
}

fn logical(block: BlockID) -> LogicalBlockID {
    LogicalBlockID {
        block,
        direction: BlockDirection::Aligned,
        facing: Facing::Forward,
    }
}

fn two_block_ids() -> (BlockID, BlockID) {
    let t0 = TrackID::new(CellID::new(0, 0, 0), Orientation::EW);
    let t1 = TrackID::new(CellID::new(1, 0, 0), Orientation::EW);
    let t3 = TrackID::new(CellID::new(3, 0, 0), Orientation::EW);
    let t4 = TrackID::new(CellID::new(4, 0, 0), Orientation::EW);
    (BlockID::new(t0, t1), BlockID::new(t3, t4))
}

fn train_entity(app: &App) -> Entity {
    app.world()
        .resource::<Registry<Train>>()
        .get(&TrainID(0))
        .unwrap()
}

/// Number of legs in the train's queue (zero when the relationship target is absent).
fn queue_len(app: &App) -> usize {
    app.world()
        .get::<TrainLegs>(train_entity(app))
        .map_or(0, |legs| legs.collection().len())
}

fn position(app: &App) -> TrainPosition {
    app.world()
        .get::<TrainPosition>(train_entity(app))
        .expect("train should have a TrainPosition")
        .clone()
}

/// Spawn the layout and a fast virtual driver, place the train at A, and
/// return the A → B route.
fn placed_train_with_route(app: &mut App) -> (LogicalBlockID, LogicalBlockID, Vec<RouteLeg>) {
    spawn_two_block_layout(app);
    let (block_a, block_b) = two_block_ids();
    let (logical_a, logical_b) = (logical(block_a), logical(block_b));

    let block_data_map = extract_block_data_map(app);
    let marker_data_map = extract_marker_data_map(app);
    let logical_graph = app.world().resource::<LogicalGraph>();
    let route = build_route(
        logical_a,
        logical_b,
        logical_graph,
        &block_data_map,
        &marker_data_map,
    )
    .unwrap();

    // Very high speed so markers are crossed within a single tick.
    app.world_mut()
        .spawn(VirtualDriver::new(TrainID(0), 1_000_000.0));

    app.world_mut()
        .write_message(PlaceTrain::new(TrainID(0), logical_a));
    app.update();
    assert_eq!(position(app), TrainPosition::at(logical_a));
    assert_eq!(queue_len(app), 0);

    (logical_a, logical_b, route)
}

/// End-to-end test: the virtual driver carries a train from block A to block B.
/// On arrival the route leg is complete, the queue is empty, and the train
/// stands in B.
#[test]
fn virtual_driver_drives_route_to_completion() {
    let mut app = make_app();
    let (_, logical_b, route) = placed_train_with_route(&mut app);

    app.world_mut()
        .write_message(AppendLegs::new(TrainID(0), route));

    // Run enough updates for the full pipeline:
    // AppendLegs → dispatch → QueueDriverLeg → driver ticks → DriverMarkerHit → TrainMarkerHit → CompleteLeg
    for _ in 0..20 {
        app.update();
    }

    assert_eq!(position(&app), TrainPosition::at(logical_b));
    assert_eq!(queue_len(&app), 0, "the completed leg leaves the queue");
}

/// With an empty queue the driver stops: further ticks produce no marker hits
/// and the train keeps standing in its position block.
#[test]
fn virtual_driver_stops_with_empty_queue() {
    let mut app = make_app();
    let (_, logical_b, route) = placed_train_with_route(&mut app);

    app.world_mut()
        .write_message(AppendLegs::new(TrainID(0), route));
    for _ in 0..20 {
        app.update();
    }
    assert_eq!(position(&app), TrainPosition::at(logical_b));

    // A marker hit with an empty queue would panic in the state layer, so
    // surviving these ticks shows the driver has stopped.
    for _ in 0..20 {
        app.update();
    }
    assert_eq!(position(&app), TrainPosition::at(logical_b));
    assert_eq!(queue_len(&app), 0);
}
