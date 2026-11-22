use bevy::ecs::relationship::RelationshipTarget;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use brickrail_common::command::{
    CommandEnvelope, CommandId, EnterControlModeRequest, ExitControlModeRequest, SimulationCommand,
};
use brickrail_common::layout::block::{Block, BlockData};
use brickrail_common::layout::connection::{Connection, ConnectionGraph};
use brickrail_common::layout::logical_graph::{LogicalGraph, LogicalGraphPlugin};
use brickrail_common::layout::marker::{Marker, MarkerData};
use brickrail_common::layout::track::Track;
use brickrail_common::layout::train::Train;
use brickrail_common::layout::*;
use brickrail_common::lifecycle::*;
use brickrail_common::primitives::*;
use brickrail_common::simulation::route::{AppendLegs, LegOf, MarkerRole, RouteLeg, TrainLegs};
use brickrail_common::simulation::train_position::{
    CompleteLeg, PlaceTrain, TrainLegState, TrainMarkerHit, TrainPosition,
};
use brickrail_common::simulation::{SimulationState, SimulationStatePlugin};
use brickrail_server::ServerPlugin;
use petgraph::algo::astar;

/// Helper: write an EnterControlMode command envelope to the SubApp.
fn enter_control_mode(app: &mut App, layout: Layout) {
    layout_world_mut(app).write_message(CommandEnvelope {
        command_id: CommandId(0),
        request: SimulationCommand::EnterControlMode(EnterControlModeRequest {
            layout,
            train_positions: vec![],
        }),
    });
}

/// Helper: write an ExitControlMode command envelope to the SubApp.
fn exit_control_mode(app: &mut App) {
    layout_world_mut(app).write_message(CommandEnvelope {
        command_id: CommandId(0),
        request: SimulationCommand::ExitControlMode(ExitControlModeRequest),
    });
}

fn make_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(ServerPlugin);
    app
}

/// Access the layout SubApp world (immutable).
fn layout_world(app: &App) -> &World {
    app.sub_app(LayoutSubApp).world()
}

/// Access the layout SubApp world (mutable).
fn layout_world_mut(app: &mut App) -> &mut World {
    app.sub_app_mut(LayoutSubApp).world_mut()
}

// --- Plugin isolation tests ---
// These verify that plugin dependency chains are clean:
// each tier works without higher tiers present.

#[test]
fn tracks_only_app_works() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(ElementPlugin::<Track>::new());
    app.update(); // should not panic
}

#[test]
fn full_layout_without_simulation_works() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(ElementPlugin::<Track>::new());
    app.add_plugins(ElementPlugin::<Connection>::new());
    app.add_plugins(ElementPlugin::<Marker>::new());
    app.add_plugins(ElementPlugin::<Block>::new());
    app.add_plugins(ElementPlugin::<Train>::new());
    app.add_plugins(LogicalGraphPlugin);
    app.update(); // should not panic — no SimulationStatePlugin needed
}

#[test]
fn simulation_state_without_logic_works() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(ElementPlugin::<Track>::new());
    app.add_plugins(ElementPlugin::<Connection>::new());
    app.add_plugins(ElementPlugin::<Marker>::new());
    app.add_plugins(ElementPlugin::<Block>::new());
    app.add_plugins(ElementPlugin::<Train>::new());
    app.add_plugins(LogicalGraphPlugin);
    app.add_plugins(SimulationStatePlugin);
    app.update(); // should not panic — no SimulationLogicPlugin needed
}

/// Test helper: A* pathfinding + leg building in one call.
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

fn test_layout() -> Layout {
    let t0 = TrackID::new(CellID::new(0, 0, 0), Orientation::EW);
    let t1 = TrackID::new(CellID::new(1, 0, 0), Orientation::EW);
    let t2 = TrackID::new(CellID::new(2, 0, 0), Orientation::EW);

    Layout {
        tracks: vec![
            ElementEntry::new(t0, Default::default()),
            ElementEntry::new(t1, Default::default()),
            ElementEntry::new(t2, Default::default()),
        ],
        connections: vec![
            ElementEntry::new(t0.get_connection_to(t1).unwrap(), Default::default()),
            ElementEntry::new(t1.get_connection_to(t2).unwrap(), Default::default()),
        ],
        markers: vec![
            ElementEntry::new(t0, Default::default()),
            ElementEntry::new(t2, Default::default()),
        ],
        blocks: vec![ElementEntry::new(
            BlockID::new(t0, t2),
            brickrail_common::layout::block::BlockData {
                name: Some("Main".to_string()),
                section: vec![
                    t0.get_directed_to(Cardinal::E).unwrap(),
                    t1.get_directed_to(Cardinal::E).unwrap(),
                    t2.get_directed_to(Cardinal::W).unwrap(),
                ],
                ..Default::default()
            },
        )],
        trains: vec![ElementEntry::new(TrainID(0), Default::default())],
    }
}

#[test]
fn enter_control_mode_spawns_layout() {
    let mut app = make_app();
    let layout = test_layout();

    enter_control_mode(&mut app, layout);
    // Command queue dispatch + state machine: intake → dispatch → handler → Entering → spawn → Running
    for _ in 0..5 {
        app.update();
    }

    let world = layout_world(&app);

    // All tracks should be spawned and registered
    let registry = world.resource::<Registry<Track>>();
    assert_eq!(registry.len(), 3);

    // Connections from layout should be spawned
    let conn_registry = world.resource::<Registry<Connection>>();
    assert_eq!(conn_registry.len(), 2);

    // Connection graph should reflect the topology
    let graph = world.resource::<ConnectionGraph>();
    assert_eq!(graph.graph.node_count(), 3);
    assert_eq!(graph.graph.edge_count(), 2);
    // Middle track should have 2 connections
    let t1 = TrackID::new(CellID::new(1, 0, 0), Orientation::EW);
    assert_eq!(graph.connections_from(t1).len(), 2);

    // Markers should be spawned
    let marker_registry = world.resource::<Registry<Marker>>();
    assert_eq!(marker_registry.len(), 2);

    // Block should be spawned with correct data
    let block_registry = world.resource::<Registry<Block>>();
    assert_eq!(block_registry.len(), 1);
    let t0 = TrackID::new(CellID::new(0, 0, 0), Orientation::EW);
    let t2 = TrackID::new(CellID::new(2, 0, 0), Orientation::EW);
    let block_entity = block_registry.get(&BlockID::new(t0, t2)).unwrap();
    let block_data = world.get::<ElementData<Block>>(block_entity).unwrap();
    assert_eq!(block_data.section.len(), 3);
    assert_eq!(block_data.name, Some("Main".to_string()));

    // Train should be spawned
    let train_registry = world.resource::<Registry<Train>>();
    assert_eq!(train_registry.len(), 1);

    // Logical graph should be built (runs in Last schedule)
    // 3 tracks × 2 directions × 2 facings = 12 logical track nodes
    // But only nodes with edges are in the graph.
    // 2 connections × 4 directed edges = 8 normal edges
    // 1 block × 2 flip edges (bidirectional) = 4 flip edges
    let logical_graph = world.resource::<LogicalGraph>();
    assert_eq!(logical_graph.graph.edge_count(), 12);

    // State should be Running after going through Entering.
    let state = layout_world(&app).resource::<State<SimulationState>>();
    assert_eq!(*state.get(), SimulationState::Running);
}

#[test]
fn exit_control_mode_cleans_up() {
    let mut app = make_app();
    let layout = test_layout();

    // Enter
    enter_control_mode(&mut app, layout);
    for _ in 0..5 {
        app.update();
    }

    // Exit
    exit_control_mode(&mut app);
    for _ in 0..5 {
        app.update();
    }

    let world = layout_world(&app);

    // Registries should be empty
    let registry = world.resource::<Registry<Track>>();
    assert_eq!(registry.len(), 0);
    let conn_registry = world.resource::<Registry<Connection>>();
    assert_eq!(conn_registry.len(), 0);
    let graph = world.resource::<ConnectionGraph>();
    assert_eq!(graph.graph.edge_count(), 0);
    let marker_registry = world.resource::<Registry<Marker>>();
    assert_eq!(marker_registry.len(), 0);
    let block_registry = world.resource::<Registry<Block>>();
    assert_eq!(block_registry.len(), 0);
    let train_registry = world.resource::<Registry<Train>>();
    assert_eq!(train_registry.len(), 0);

    // Cascade-despawned dependents should be gone too
    {
        let world = layout_world_mut(&mut app);
        let driver_count = world
            .query::<&brickrail_common::simulation::virtual_driver::VirtualDriver>()
            .iter(world)
            .count();
        assert_eq!(
            driver_count, 0,
            "VirtualDrivers should be cascade-despawned"
        );
        let leg_count = world.query::<&RouteLeg>().iter(world).count();
        assert_eq!(leg_count, 0, "RoutLegs should be cascade-despawned");
    }

    // State should be back to Idle
    let state = layout_world(&app).resource::<State<SimulationState>>();
    assert_eq!(*state.get(), SimulationState::Idle);
}

#[test]
fn round_trip_enter_exit_enter() {
    let mut app = make_app();
    let layout = test_layout();

    // First enter
    enter_control_mode(&mut app, layout.clone());
    for _ in 0..5 {
        app.update();
    }
    assert_eq!(layout_world(&app).resource::<Registry<Track>>().len(), 3);

    // Exit
    exit_control_mode(&mut app);
    for _ in 0..5 {
        app.update();
    }
    assert_eq!(layout_world(&app).resource::<Registry<Track>>().len(), 0);

    // Second enter
    enter_control_mode(&mut app, layout);
    for _ in 0..5 {
        app.update();
    }
    assert_eq!(layout_world(&app).resource::<Registry<Track>>().len(), 3);
}

/// Layout with two blocks and a travel section between them:
/// t0 -- t1 -- t2 -- t3 -- t4
/// [block A]  travel  [block B]
fn two_block_layout() -> Layout {
    let t0 = TrackID::new(CellID::new(0, 0, 0), Orientation::EW);
    let t1 = TrackID::new(CellID::new(1, 0, 0), Orientation::EW);
    let t2 = TrackID::new(CellID::new(2, 0, 0), Orientation::EW);
    let t3 = TrackID::new(CellID::new(3, 0, 0), Orientation::EW);
    let t4 = TrackID::new(CellID::new(4, 0, 0), Orientation::EW);

    Layout {
        tracks: vec![
            ElementEntry::new(t0, Default::default()),
            ElementEntry::new(t1, Default::default()),
            ElementEntry::new(t2, Default::default()),
            ElementEntry::new(t3, Default::default()),
            ElementEntry::new(t4, Default::default()),
        ],
        connections: vec![
            ElementEntry::new(t0.get_connection_to(t1).unwrap(), Default::default()),
            ElementEntry::new(t1.get_connection_to(t2).unwrap(), Default::default()),
            ElementEntry::new(t2.get_connection_to(t3).unwrap(), Default::default()),
            ElementEntry::new(t3.get_connection_to(t4).unwrap(), Default::default()),
        ],
        markers: vec![
            ElementEntry::new(t0, Default::default()),
            ElementEntry::new(t1, Default::default()),
            ElementEntry::new(t3, Default::default()),
            ElementEntry::new(t4, Default::default()),
        ],
        blocks: vec![
            ElementEntry::new(
                BlockID::new(t0, t1),
                BlockData {
                    name: Some("A".to_string()),
                    section: vec![
                        t0.get_directed_to(Cardinal::E).unwrap(),
                        t1.get_directed_to(Cardinal::E).unwrap(),
                    ],
                    ..Default::default()
                },
            ),
            ElementEntry::new(
                BlockID::new(t3, t4),
                BlockData {
                    name: Some("B".to_string()),
                    section: vec![
                        t3.get_directed_to(Cardinal::E).unwrap(),
                        t4.get_directed_to(Cardinal::E).unwrap(),
                    ],
                    ..Default::default()
                },
            ),
        ],
        trains: vec![ElementEntry::new(TrainID(0), Default::default())],
    }
}

#[test]
fn build_route_between_two_blocks() {
    let mut app = make_app();
    let layout = two_block_layout();

    enter_control_mode(&mut app, layout);
    for _ in 0..5 {
        app.update();
    }

    let t0 = TrackID::new(CellID::new(0, 0, 0), Orientation::EW);
    let t1 = TrackID::new(CellID::new(1, 0, 0), Orientation::EW);
    let t2 = TrackID::new(CellID::new(2, 0, 0), Orientation::EW);
    let t3 = TrackID::new(CellID::new(3, 0, 0), Orientation::EW);
    let t4 = TrackID::new(CellID::new(4, 0, 0), Orientation::EW);

    let block_a = BlockID::new(t0, t1);
    let block_b = BlockID::new(t3, t4);

    let world = layout_world(&app);

    // Build block data map
    let block_registry = world.resource::<Registry<Block>>();
    let mut block_data_map = HashMap::new();
    for (id, &entity) in block_registry.iter() {
        let data = world.get::<ElementData<Block>>(entity).unwrap();
        block_data_map.insert(*id, data.0.clone());
    }

    // Build marker data map
    let marker_registry = world.resource::<Registry<Marker>>();
    let mut marker_data_map = HashMap::new();
    for (id, &entity) in marker_registry.iter() {
        let data = world.get::<ElementData<Marker>>(entity).unwrap();
        marker_data_map.insert(*id, data.0.clone());
    }

    let logical_graph = world.resource::<LogicalGraph>();

    let start = LogicalBlockID {
        block: block_a,
        direction: BlockDirection::Aligned,
        facing: Facing::Forward,
    };
    let target = LogicalBlockID {
        block: block_b,
        direction: BlockDirection::Aligned,
        facing: Facing::Forward,
    };

    let legs = build_route(
        start,
        target,
        logical_graph,
        &block_data_map,
        &marker_data_map,
    )
    .expect("should find a route");

    assert_eq!(legs.len(), 1);

    let leg = &legs[0];
    assert_eq!(leg.facing, Facing::Forward);
    assert_eq!(leg.start_block.block_id, block_a);
    assert_eq!(leg.target_block.block_id, block_b);

    // Start block section should contain the block A tracks
    assert_eq!(leg.start_block.section.len(), 2);
    assert_eq!(leg.start_block.section[0].track, t0);
    assert_eq!(leg.start_block.section[1].track, t1);

    // Travel section should contain t2
    assert_eq!(leg.travel.len(), 1);
    assert_eq!(leg.travel[0].track, t2);

    // Target block section should contain block B tracks
    assert_eq!(leg.target_block.section.len(), 2);
    assert_eq!(leg.target_block.section[0].track, t3);
    assert_eq!(leg.target_block.section[1].track, t4);

    // Markers along sensor trajectory: t1 (Exiting), t3 (Entering), t4 (Entered)
    // t0 is not on the sensor path (A* starts at enter marker t1)
    // t2 has no marker in this layout
    assert_eq!(leg.markers.len(), 3);
    assert_eq!(leg.markers[0].track, t1);
    assert_eq!(leg.markers[0].role, Some(MarkerRole::Exiting));
    assert_eq!(leg.markers[1].track, t3);
    assert_eq!(leg.markers[1].role, Some(MarkerRole::Entering));
    assert_eq!(leg.markers[2].track, t4);
    assert_eq!(leg.markers[2].role, Some(MarkerRole::Entered));

    // Positions are marker-count units (0, 1, 2, ...)
    assert_eq!(leg.markers[0].position, 0.0);
    assert_eq!(leg.markers[1].position, 1.0);
    assert_eq!(leg.markers[2].position, 2.0);
}

#[test]
fn append_legs_spawns_leg_entities() {
    let mut app = make_app();
    let layout = two_block_layout();

    enter_control_mode(&mut app, layout);
    for _ in 0..5 {
        app.update();
    }

    let t0 = TrackID::new(CellID::new(0, 0, 0), Orientation::EW);
    let t1 = TrackID::new(CellID::new(1, 0, 0), Orientation::EW);
    let t3 = TrackID::new(CellID::new(3, 0, 0), Orientation::EW);
    let t4 = TrackID::new(CellID::new(4, 0, 0), Orientation::EW);

    let block_a = BlockID::new(t0, t1);
    let block_b = BlockID::new(t3, t4);

    // Build legs externally (as strategy code would)
    let world = layout_world(&app);
    let block_registry = world.resource::<Registry<Block>>();
    let mut block_data_map = HashMap::new();
    for (id, &entity) in block_registry.iter() {
        let data = world.get::<ElementData<Block>>(entity).unwrap();
        block_data_map.insert(*id, data.0.clone());
    }
    let marker_registry = world.resource::<Registry<Marker>>();
    let mut marker_data_map = HashMap::new();
    for (id, &entity) in marker_registry.iter() {
        let data = world.get::<ElementData<Marker>>(entity).unwrap();
        marker_data_map.insert(*id, data.0.clone());
    }
    let logical_graph = world.resource::<LogicalGraph>();

    let start = LogicalBlockID {
        block: block_a,
        direction: BlockDirection::Aligned,
        facing: Facing::Forward,
    };
    let target = LogicalBlockID {
        block: block_b,
        direction: BlockDirection::Aligned,
        facing: Facing::Forward,
    };

    let legs = build_route(
        start,
        target,
        logical_graph,
        &block_data_map,
        &marker_data_map,
    )
    .expect("should find a route");

    // Place the train at the start block, then append the legs via state event message
    layout_world_mut(&mut app).write_message(PlaceTrain::new(TrainID(0), start));
    app.update();
    layout_world_mut(&mut app).write_message(AppendLegs::new(TrainID(0), legs));
    app.update();

    // Resolve train entity from registry for assertions
    let world = layout_world(&app);
    let train_registry = world.resource::<Registry<Train>>();
    let train_entity = train_registry.get(&TrainID(0)).unwrap();

    // Query spawned leg entities
    let sub_app = app.sub_app_mut(LayoutSubApp);
    let spawned: Vec<_> = sub_app
        .world_mut()
        .query::<(&RouteLeg, &LegOf)>()
        .iter(sub_app.world())
        .collect();

    assert_eq!(spawned.len(), 1);

    let (leg, leg_of) = spawned[0];
    assert_eq!(leg_of.0, train_entity);
    assert_eq!(leg.start_block.block_id, block_a);
    assert_eq!(leg.target_block.block_id, block_b);
    assert_eq!(leg.facing, Facing::Forward);
    assert_eq!(leg.markers.len(), 3);
}

// --- Helper: extract data maps from ECS ---

fn extract_block_data_map(app: &App) -> HashMap<BlockID, BlockData> {
    let world = layout_world(app);
    let block_registry = world.resource::<Registry<Block>>();
    let mut map = HashMap::new();
    for (id, &entity) in block_registry.iter() {
        let data = world.get::<ElementData<Block>>(entity).unwrap();
        map.insert(*id, data.0.clone());
    }
    map
}

fn extract_marker_data_map(app: &App) -> HashMap<TrackID, MarkerData> {
    let world = layout_world(app);
    let marker_registry = world.resource::<Registry<Marker>>();
    let mut map = HashMap::new();
    for (id, &entity) in marker_registry.iter() {
        let data = world.get::<ElementData<Marker>>(entity).unwrap();
        map.insert(*id, data.0.clone());
    }
    map
}

// --- Train position tests ---

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
    layout_world(app)
        .resource::<Registry<Train>>()
        .get(&TrainID(0))
        .unwrap()
}

/// Number of legs in the train's queue (zero when the relationship target is absent).
fn queue_len(app: &App) -> usize {
    layout_world(app)
        .get::<TrainLegs>(train_entity(app))
        .map_or(0, |legs| legs.collection().len())
}

fn front_leg(app: &App) -> RouteLeg {
    let world = layout_world(app);
    let legs = world.get::<TrainLegs>(train_entity(app)).unwrap();
    world
        .get::<RouteLeg>(*legs.collection().first().unwrap())
        .unwrap()
        .clone()
}

fn position(app: &App) -> TrainPosition {
    layout_world(app)
        .get::<TrainPosition>(train_entity(app))
        .expect("train should have a TrainPosition")
        .clone()
}

/// Enter control mode, place the train at block A, and build the A → B route.
fn placed_train_with_route(app: &mut App) -> (LogicalBlockID, LogicalBlockID, Vec<RouteLeg>) {
    enter_control_mode(app, two_block_layout());
    for _ in 0..5 {
        app.update();
    }
    let (block_a, block_b) = two_block_ids();
    let (logical_a, logical_b) = (logical(block_a), logical(block_b));

    layout_world_mut(app).write_message(PlaceTrain::new(TrainID(0), logical_a));
    app.update();

    let block_data_map = extract_block_data_map(app);
    let marker_data_map = extract_marker_data_map(app);
    let logical_graph = layout_world(app).resource::<LogicalGraph>();
    let route = build_route(
        logical_a,
        logical_b,
        logical_graph,
        &block_data_map,
        &marker_data_map,
    )
    .expect("should find route");
    (logical_a, logical_b, route)
}

#[test]
fn place_train_creates_position() {
    let mut app = make_app();
    enter_control_mode(&mut app, two_block_layout());
    for _ in 0..5 {
        app.update();
    }
    let (block_a, _) = two_block_ids();

    layout_world_mut(&mut app).write_message(PlaceTrain::new(TrainID(0), logical(block_a)));
    app.update();

    assert_eq!(position(&app), TrainPosition::at(logical(block_a)));
    assert_eq!(queue_len(&app), 0, "a placed train has an empty queue");
}

#[test]
fn append_legs_continues_from_position_block() {
    let mut app = make_app();
    let (logical_a, logical_b, route) = placed_train_with_route(&mut app);

    layout_world_mut(&mut app).write_message(AppendLegs::new(TrainID(0), route));
    app.update();

    // Appending leaves the position untouched; the route becomes the front leg.
    assert_eq!(position(&app), TrainPosition::at(logical_a));
    assert_eq!(queue_len(&app), 1);
    let leg = front_leg(&app);
    assert_eq!(leg.start_logical_block_id(), logical_a);
    assert_eq!(leg.target_logical_block_id(), logical_b);
}

#[test]
#[should_panic(expected = "must start where the queue ends")]
fn append_legs_rejects_leg_not_starting_at_position_block() {
    let mut app = make_app();
    let (_, logical_b, route) = placed_train_with_route(&mut app);

    // Re-place the train at B, then try to append a route that starts at A.
    layout_world_mut(&mut app).write_message(PlaceTrain::new(TrainID(0), logical_b));
    app.update();
    layout_world_mut(&mut app).write_message(AppendLegs::new(TrainID(0), route));
    app.update();
}

#[test]
fn marker_hit_increments_and_updates_state() {
    let mut app = make_app();
    let (_, _, route) = placed_train_with_route(&mut app);
    layout_world_mut(&mut app).write_message(AppendLegs::new(TrainID(0), route));
    app.update();

    // Route leg has 3 markers: [0]=Exiting, [1]=Entering, [2]=Entered.
    // The train starts at marker_index=0 (already past the Exiting marker).
    let leg = front_leg(&app);
    assert_eq!(position(&app).leg_state(&leg), TrainLegState::ExitingStart);

    layout_world_mut(&mut app).write_message(TrainMarkerHit::new(TrainID(0)));
    app.update();
    let pos = position(&app);
    assert_eq!(pos.marker_index, 1);
    assert_eq!(pos.leg_state(&leg), TrainLegState::EnteringTarget);
    assert!(!pos.has_completed(&leg));

    layout_world_mut(&mut app).write_message(TrainMarkerHit::new(TrainID(0)));
    app.update();
    let pos = position(&app);
    assert_eq!(pos.marker_index, 2);
    assert_eq!(pos.leg_state(&leg), TrainLegState::EnteredTarget);
    assert!(pos.has_completed(&leg));
}

#[test]
fn complete_leg_pops_front_leg_and_moves_position_block() {
    let mut app = make_app();
    let (_, logical_b, route) = placed_train_with_route(&mut app);
    layout_world_mut(&mut app).write_message(AppendLegs::new(TrainID(0), route));
    app.update();

    layout_world_mut(&mut app).write_message(CompleteLeg::new(TrainID(0)));
    app.update();

    assert_eq!(position(&app), TrainPosition::at(logical_b));
    assert_eq!(queue_len(&app), 0);

    let sub_app = app.sub_app_mut(LayoutSubApp);
    let legs_count = sub_app
        .world_mut()
        .query::<&RouteLeg>()
        .iter(sub_app.world())
        .count();
    assert_eq!(legs_count, 0, "the completed leg entity is despawned");
}
